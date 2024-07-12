// Note: It works as expected for number up to Number.MAX_SAFE_INTEGER=2^53-1
function getRandom(min, max) {
    min = BigInt(min);
    max = BigInt(max);

    if (min > max) {
        throw new Error("min must be less than or equal to max");
    }

    const range = max - min + 1n;

    const rand = BigInt(Math.floor(Math.random() * Number(range)));

    return rand + min;
}

module.exports = { getRandom };