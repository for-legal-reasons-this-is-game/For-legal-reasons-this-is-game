function _base64Encode(data) {
    if (!data) return data;
    return btoa(encodeURIComponent(data).replace(/%([0-9A-F]{2})/g, (_, p1)=>String.fromCharCode(parseInt(p1, 16))));
}
export { _base64Encode };
