var http = require('http'),
    fs = require('fs'),
    path = require('path');

function fourOhFour (req, res, message) {
    console.log('X--- ' + req.url + ' / ' + message);
    res.writeHead(404);
    res.end("" + message);
}

function augmentFiles (files, dir) {
    files.unshift('..');
    files.unshift('.');
    files = files.map(function (file) {
        var stats = fs.statSync(dir + file);
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
            fullPath: file
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
        var stat = fs.statSync(filePath);

        console.log('<--- index.html');
        res.writeHead(200, {
            'Content-Type': 'text/html',
            'Content-Length': stat.size
        });
        var readStream = fs.createReadStream(filePath);
        // We replaced all the event handlers with a simple call to readStream.pipe()
        readStream.pipe(res);

    } else if (match && match[1]) {
        // render dir api

        var matchDir = unescape(match[1]);
        console.log('   > dir=' + matchDir);

        var files;
        try {
            files = augmentFiles(fs.readdirSync(matchDir), matchDir);
        } catch (e) {
            fourOhFour(req, res, e);
        }

        if (!files || files.length === 0) {
            fourOhFour(req, res);
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
    }

}).listen(9333, 'localhost');

console.log('Server running at http://localhost:9333/');