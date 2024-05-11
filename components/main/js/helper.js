const Helper = require(__dirname + '../../src/helper.js');
module.exports = class extends Helper {
    constructor(config = {}) {
        super(config);
    }   
    eval_dumpRegs(cmd) {
        // TODO:
        TODO
        console.log(`dumpRegs ${this.sourceRef}`);
        if (ctx.fullFe) {
            console.log(['A', fea2scalar(ctx.Fr, ctx.A)]);
            console.log(['B', fea2scalar(ctx.Fr, ctx.B)]);
            console.log(['C', fea2scalar(ctx.Fr, ctx.C)]);
            console.log(['D', fea2scalar(ctx.Fr, ctx.D)]);
            console.log(['E', fea2scalar(ctx.Fr, ctx.E)]);
        } else {
            console.log(['A', safeFea2scalar(ctx.Fr, ctx.A)]);
            console.log(['B', safeFea2scalar(ctx.Fr, ctx.B)]);
            console.log(['C', safeFea2scalar(ctx.Fr, ctx.C)]);
            console.log(['D', safeFea2scalar(ctx.Fr, ctx.D)]);
            console.log(['E', safeFea2scalar(ctx.Fr, ctx.E)]);
        }

        return [ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero];
    }
    eval_dump(cmd) {
        // TODO:
        TODO
        console.log("\x1b[38;2;175;175;255mDUMP on " + ctx.fileName + ":" + ctx.line+"\x1b[0m");
    
        tag.params.forEach((value) => {
            let name = value.varName || value.paramName || value.regName || value.offsetLabel;
            if (typeof name == 'undefined' && value.path) {
                name = value.path.join('.');
            }
            console.log("\x1b[35m"+ name +"\x1b[0;35m: "+evalCommand(ctx, value)+"\x1b[0m");
        });

        return [ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero];
    }
    eval_dumphex(ctx, tag) {
         // TODO:
        TODO
        console.log("\x1b[38;2;175;175;255mDUMP on " + ctx.fileName + ":" + ctx.line+"\x1b[0m");

        tag.params.forEach((value) => {
            let name = value.varName || value.paramName || value.regName;
            if (typeof name == 'undefined' && value.path) {
                name = value.path.join('.');
            }
            console.log("\x1b[35m"+ name +"\x1b[0;35m: 0x"+evalCommand(ctx, value).toString(16)+"\x1b[0m");
        });
    
        return [ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero];
    }
}