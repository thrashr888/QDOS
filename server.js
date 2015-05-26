var http = require('http'),
    fs = require('fs'),
    path = require('path'),
    stream = require('stream'),
    util = require('util');

var STARTDIR = path.normalize(__dirname + path.sep + process.argv[2]);

function sanitize(input) {
    var illegalRe = /[\?<>\\:\*\|":]/g;
    var controlRe = /[\x00-\x1f\x80-\x9f]/g;
    var reservedRe = /^\.+$/;
    var windowsReservedRe = /^(con|prn|aux|nul|com[0-9]|lpt[0-9])(\..*)?$/i;

    return input
        .replace(illegalRe, '')
        .replace(controlRe, '')
        .replace(reservedRe, '')
        .replace(windowsReservedRe, '');
}

function fileToPath (file, dir) {
    return path.resolve(dir, file);
}

function pathToFile (filePath) {
    return path.basename(filePath);
}

function fourOhFour (req, res, message) {
    console.log('X--- ' + req.url + ' / ' + message);
    res.writeHead(404);
    res.end("" + message);
}

function augmentFiles (files, dir) {
    files.unshift('..');
    files.unshift('.');
    files = files.map(function (file) {
        var path = fileToPath(file, dir);
        var stats = fs.statSync(path);
        return {
            fileName: file,
            size: stats.size,
            date: stats.mtime,
            time: stats.mtime,
            datetime: stats.mtime,
            atime: stats.atime,
            mtime: stats.mtime,
            ctime: stats.ctime,
            birthtime: stats.birthtime,
            isDir: stats.isDirectory(),
            isDirectory: stats.isDirectory(),
            isFile: stats.isFile(),
            // isLink: stats.isSymbolicLink(),
            fullPath: path
        };
    });
    return files;
}


function StrReplace(find, replace) {
    // allow use without new
    if (!(this instanceof StrReplace)) {
        return new StrReplace(find, replace);
    }

    // init Transform
    stream.Transform.call(this, {find: find, replace: replace});
    this.find = find;
    this.replace = replace;
}
util.inherits(StrReplace, stream.Transform);

StrReplace.prototype._transform = function (chunk, enc, cb) {
    var upperChunk = chunk.toString().replace(this.find, this.replace);
    this.push(upperChunk, enc);
    cb();
};


http.createServer(function Server (req, res) {
    // console.log('req', req);

    // /?dir=/
    var match = req.url.match(/dir=(.*)/i);

    console.log('---> ', req.url);

    if (req.url === '/') {
        // render index.html

        var filePath = path.join(__dirname, 'index.html');
        var stat = fs.statSync(filePath);

        // TODO replace %START_DIR% with argv[1]

        console.log('<--- index.html');
        res.writeHead(200, {
            'Content-Type': 'text/html',
            'Content-Length': stat.size
        });
        var readStream = fs.createReadStream(filePath);
        // We replaced all the event handlers with a simple call to readStream.pipe()
        var repl = new StrReplace('%START_DIR%', STARTDIR);
        readStream.pipe(repl).pipe(res);

    } else if (match && match[1]) {
        // render dir api

        var matchDir = path.normalize(sanitize(unescape(match[1])));
        console.log('   > dir=' + matchDir);

        var files;
        try {
            files = augmentFiles(fs.readdirSync(matchDir), matchDir);
        } catch (e) {
            fourOhFour(req, res, e);
            return;
        }

        if (!files || files.length === 0) {
            fourOhFour(req, res);
            return;
        }

        console.log('   > files.length=' + files.length);
        var text = JSON.stringify({files: files});
        res.writeHead(200, {
            'Content-Type': 'application/json',
            'Content-Length': text.length
        });
        res.end(text);

    } else {
        fourOhFour(req, res);
        return;
    }

}).listen(9333, 'localhost');

console.log('Server running at http://localhost:9333/');
console.log('Serving ' + STARTDIR);