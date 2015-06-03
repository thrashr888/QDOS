var http = require('http'),
    fs = require('fs'),
    path = require('path'),
    stream = require('stream'),
    util = require('util'),
    os = require('os'),
    child_process = require('child_process'),
    url = require('url'),
    querystring = require('querystring')
    ;

var ROOT_DIR = path.resolve(process.argv[2]);

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

function parseDiskutil (out) {
    out.split('\n').filter(function (line) {
        return line.match(/^\s*\d+:/);
    }).map(function (line) {
        var m = line.match(/^\s*\d+:\s*[\w_]+\s([\w\s\_\-]*)\s*\d/);
        return match ? m[1] : null;
    });
    return out;
}

function fourOhFour (req, res, message) {
    console.log('X--- ' + req.url + ' / ' + message);
    res.writeHead(404);
    res.end("" + message);
}

function serverError (req, res, message) {
    console.log('X--- ' + req.url + ' / ' + message);
    res.writeHead(500);
    res.end("" + message);
}

function augmentFiles (files, dir) {
    files.unshift('..');
    files.unshift('.');
    files = files.map(function (file) {
        var path = fileToPath(file, dir);
        var stats = fs.statSync(path);
        var extMatch = file.match(/.*\.(\w+)$/i);
        return {
            path: path,
            fullPath: path,
            dir: dir,
            name: file,
            ext: extMatch && extMatch[1] ? extMatch[1] : null,
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
        };
    });
    return files;
}

function handleDirApi (req, res, qs) {
    // render dir api
    var dir = path.normalize(sanitize(unescape(qs.dir)));

    var files;
    try {
        files = augmentFiles(fs.readdirSync(dir), dir);
    } catch (e) {
        fourOhFour(req, res, e);
        return;
    }

    if (!files || files.length === 0) {
        fourOhFour(req, res);
        return;
    }

    console.log('<--- dir=' + dir, ', files.length=' + files.length);
    var text = JSON.stringify({
        dir: dir,
        files: files,
    });
    res.writeHead(200, {
        'Content-Type': 'application/json',
        'Content-Length': text.length
    });
    res.end(text);
}

function handleViewApi (req, res, qs) {

}

function handleCopyApi (req, res, qs) {

}

function handleMoveApi (req, res, qs) {

}

function handleFindApi (req, res, qs) {

}

function handleEraseApi (req, res, qs) {

}

function handleRenameApi (req, res, qs) {

}

function handleStatusApi (req, res, qs) {
    child_process.execFile('diskutil', ['list'], function(error, stdout, stderr) {
        if (err) {
            serverError(req, res);
            return;
        }

        var text = JSON.stringify({
            status: {
                ram: { used: os.freemem(), total: os.totalmem() },
                drives: parseDiskutil(stdout),
            }
        });
        console.log('<--- status', text.mem);
        res.writeHead(200, {
            'Content-Type': 'application/json',
            'Content-Length': text.length
        });
        res.end(text);
    });
}


http.createServer(function Server (req, res) {
    // console.log('req', req);

    // /?dir=/
    var match = req.url.match(/dir=(.*)/i);
    var u = url.parse(req.url);
    var qs = querystring.parse(u.query);

    console.log('---> ', req.url, '~', u.pathname, qs);

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

    } else if (qs) {
        if (qs.dir) {
            // render Dir api
            return handleDirApi(req, res, qs);
        } else if (qs.view) {
            // render View api
            return handleViewApi(req, res, qs);
        } else if (qs.copy) {
            // render Copy api
            return handleCopyApi(req, res, qs);
        } else if (qs.move) {
            // render Move api
            return handleMoveApi(req, res, qs);
        } else if (qs.find) {
            // render Find api
            return handleFindApi(req, res, qs);
        } else if (qs.erase) {
            // render Erase api
            return handleEraseApi(req, res, qs);
        } else if (qs.rename) {
            // render Rename api
            return handleRenameApi(req, res, qs);
        } else if (qs.status) {
            // render Rename api
            return handleStatusApi(req, res, qs);
        }
    } else {
        fourOhFour(req, res);
        return;
    }

}).listen(9333, 'localhost');

console.log('Server running at http://localhost:9333/');
console.log('Serving ' + ROOT_DIR);