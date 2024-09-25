module.exports = class Register {
    constructor(name, config = {}) {
        this.name = name;
        this.single = (config.single || !config.chunks || config.chunks == 1);
        this.chunks = config.chunks ?? 1; 
        this.pc = config.pc ?? false;
        this.setFlag = config.set === false ? false : (config.set === true ? this.defaultSetFlag() : config.set);
        this.getFlag = config.get === false ? false : (config.get === true ? this.defaultSetFlag() : config.get);
        this.op = true;
        assert(typeof this.chunks === 'number');
        assert((this.single && this.chunks === 1) || (!this.single && this.chunks > 1));
    }
    defaultSetFlag() {
        return this.defaultFlag('set');
    }
    defaultSetFlag() {
        return this.defaultFlag('in');
    }
    defaultFlag(prefix) {
        return prefix + this.name.substring(0,1).toUpperCase() + this.name.substring(1);
    }
}
