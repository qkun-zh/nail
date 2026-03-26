import {valid} from 'semver';
export function match(param) {
    return valid(param);
}