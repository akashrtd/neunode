"use strict";

const { generateBinPath } = require("./platform");

const version = "0.1.0";
const binPath = generateBinPath();

module.exports = { binPath, version };
