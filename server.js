var http = require('http'),
    fs = require('fs'),
    path = require('path'),
    stream = require('stream'),
    util = require('util');

var ROOT_DIR = path.normalize(process.argv[2]);

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


http.createServer(function Server (req, res) {
    // console.log('req', req);

    // /?dir=/
    var match = req.url.match(/dir=(.*)/i);

    console.log('---> ', req.url);

    if (req.url === '/') {
        // render index.html

        var filePath = path.join(__dirname, 'index.html');
        var html = fs.readFileSync(filePath, {encoding: 'utf8'});
        html = html.replace('%ROOT_DIR%', ROOT_DIR);
        html = html.replace('%PATH_SEP%', path.sep);

        console.log('<--- index.html');
        res.writeHead(200, {
            'Content-Type': 'text/html',
            'Content-Length': html.length
        });
        res.end(html);

    } else if (match && match[1]) {
        // render dir api

        var matchDir = path.normalize(sanitize(unescape(match[1])));

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

        console.log('   > dir=' + matchDir);
        console.log('   > files.length=' + files.length);
        var text = JSON.stringify({
            dir: matchDir,
            files: files,
        });
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
console.log('Serving ' + ROOT_DIR);