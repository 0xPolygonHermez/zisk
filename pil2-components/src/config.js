const yaml = require('yaml');
const fs = require('fs');


const file = fs.readFileSync(__dirname + '/config.yml', 'utf8')
console.log(yaml.parse(file));