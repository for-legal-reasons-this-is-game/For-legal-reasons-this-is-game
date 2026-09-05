import { document as external_globals_mjs_document } from "./globals.mjs";
const getCookieValue = (name)=>{
    if (!external_globals_mjs_document) return;
    try {
        const nameEQ = name + '=';
        const cookies = external_globals_mjs_document.cookie.split(';').filter((cookie)=>cookie.length);
        for(let i = 0; i < cookies.length; i++){
            let cookie = cookies[i];
            while(' ' == cookie.charAt(0))cookie = cookie.substring(1, cookie.length);
            if (0 === cookie.indexOf(nameEQ)) return decodeURIComponent(cookie.substring(nameEQ.length, cookie.length));
        }
    } catch  {}
    return null;
};
export { getCookieValue };
