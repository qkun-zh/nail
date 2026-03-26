import {v7 as uuidV7} from "uuid";
import { version as uuidVersion } from 'uuid';
import { validate as uuidValidate } from 'uuid';


function uuidValidateV7(uuid) {
    return uuidValidate(uuid) && uuidVersion(uuid) === 7;
}
export function match(param) {
    // return ;
    return uuidValidateV7(param)||/^[0-9a-f]{32}$/.test(param)
}