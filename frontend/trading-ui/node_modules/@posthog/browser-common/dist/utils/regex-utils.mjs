const isValidRegex = function(str) {
    try {
        new RegExp(str);
    } catch  {
        return false;
    }
    return true;
};
const isMatchingRegex = function(value, pattern) {
    if (!isValidRegex(pattern)) return false;
    try {
        return new RegExp(pattern).test(value);
    } catch  {
        return false;
    }
};
export { isMatchingRegex, isValidRegex };
