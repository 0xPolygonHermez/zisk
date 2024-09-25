/* lexical grammar */
%lex
%%
\;[^\n\r]*              { /* console.log("COMMENT: "+yytext) */ }
\/\*([^*]|[\r\n]|(\*+([^*/]|[\r\n])))*\*+\/ { /* console.log("MULTILINE COMMENT: "+yytext); */  }
((0x[0-9A-Fa-f][0-9A-Fa-f_]*)|([0-9][0-9_]*))n          { yytext = BigInt(yytext.replace(/[\_n]/g, "")); return 'NUMBERL'; }
(0x[0-9A-Fa-f][0-9A-Fa-f_]*)|([0-9][0-9_]*)          { yytext = Number(yytext.replace(/\_/g, "")); return 'NUMBER'; }
\$\$\{[^\}]*\}          { yytext = yytext.slice(3, -1); return "COMMAND"; }
(\$0(\{[^\}]*\})?)      { yytext = yytext.length == 2 ? "" : yytext.slice(3, -1); return 'TAG_0'; }
(\$BYTE(\{[^\}]*\})?)      { yytext = yytext.length == 2 ? "" : yytext.slice(6, -1); return 'TAG_BYTE'; }
(\$(\{[^\}]*\})?)       { yytext = yytext.length == 1 ? "" : yytext.slice(2, -1); return 'TAG'; }
[\r\n]+                 { return "LF";}
[ \t]+                  { /* console.log("Empty spaces"); */ }
A                       { return 'A'; }
B                       { return 'B'; }
C                       { return 'C'; }
D                       { return 'D'; }
E                       { return 'E'; }
RCX                     { return 'RCX'; }
SR                      { return 'SR'; }
CTX                     { return 'CTX'; }
SP                      { return 'SP'; }
PC                      { return 'PC'; }
zkPC                    { return 'zkPC'; }
RR                      { return 'RR'; }
STEP                    { return 'STEP'; }
ROTL_C                  { return 'ROTL_C'; }
F_MLOAD                 { return 'F_MLOAD' }
MLOAD                   { return 'MLOAD' }
MSTORE                  { return 'MSTORE' }
JMPC                    { return 'JMPC' }
JMPZ                    { return 'JMPZ' }
JMPNZ                   { return 'JMPNZ' }
JMPNC                   { return 'JMPNC' }
JMPN                    { return 'JMPN' }
JMP_EQ                  { return 'JMP_EQ' }
JMP_NE                  { return 'JMP_NE' }
JMP_LT                  { return 'JMP_LT' }
JMP_LE                  { return 'JMP_LE' }
JMP_GT                  { return 'JMP_GT' }
JMP_GE                  { return 'JMP_GE' }
JMP                     { return 'JMP' }
CALL_C                  { return 'CALL_C' }
CALL_Z                  { return 'CALL_Z' }
CALL_NC                 { return 'CALL_NC' }
CALL_NZ                 { return 'CALL_NZ' }
CALL_N                  { return 'CALL_N' }
CALL_EQ                 { return 'CALL_EQ' }
CALL_NE                 { return 'CALL_NE' }
CALL_LT                 { return 'CALL_LT' }
CALL_LE                 { return 'CALL_LE' }
CALL_GT                 { return 'CALL_GT' }
CALL_GE                 { return 'CALL_GE' }
CALL                    { return 'CALL' }
RETURN                  { return 'RETURN' }
ASSERT                  { return 'ASSERT' }
SYS                     { return 'SYS' }
MEM                     { return 'MEM' }
STACK                   { return 'STACK' }
INCLUDE                 { return 'INCLUDE' }
VAR                     { return 'VAR' }
GLOBAL                  { return 'GLOBAL' }
CTX                     { return 'CTX' }
CONST                   { return 'CONST' }
CONSTL                  { return 'CONSTL' }
REPEAT                  { return 'REPEAT' }
\"[^"]+\"               { yytext = yytext.slice(1,-1); return 'STRING'; }
[a-zA-Z_][a-zA-Z$_0-9]*\:   { yytext = yytext.slice(0, -1); return 'LABEL'; }
[a-zA-Z_][a-zA-Z$_0-9]*   { return 'IDENTIFIER'; }
\%[a-zA-Z_][a-zA-Z$_0-9]* { yytext = yytext.slice(1); return 'CONSTID'; }
\@[a-zA-Z_][a-zA-Z$_0-9]* { yytext = yytext.slice(1); return 'REFERENCE'; }
\:                      { return ':'; }
\,                      { return ','}
\(                      { return '('}
\)                      { return ')'}
\+\+                    { return '++'}
\-\-                    { return '--'}
\+                      { return '+'}
\-                      { return '-'}
\/                      { return '/'}
\*\*                    { return '**'}
\*                      { return '*'}
\%                      { return '%'}
\=\>                    { return '=>' }
\<\<                    { return '<<' }
\>\>                    { return '>>' }
\^                      { return '^' }
\|\|                    { return '||' }
\&\&                    { return '&&' }
\&                      { return '&' }
\|                      { return '|' }
\=\=                    { return '==' }
\!\=                    { return '!=' }
\<\=                    { return '<=' }
\>\=                    { return '>=' }
\>                      { return '>' }
\<                      { return '<' }
\=                      { return '=' }
\!                      { return '!' }
\?\?                    { return '??' }
\?                      { return '?' }
\[                      { return '[' }
\]                      { return ']' }
<<EOF>>                 { return 'EOF'; }
.                       { /* console.log("INVALID: " + yytext); */ return 'INVALID'; }

/lex
%nonassoc EMPTY
%left '?' ':'

%right '='
%right '?'
%left '??'
%left '||'
%left '&&'
%left '|'
%left '^'
%left '&'
%left '==' '!='
%left '<' '<=' '>' '>='
%left '<<' '>>'
%left '+' '-'
%left '**' '*' '%' '/'
%right '!'
%{
const lodash = require('lodash');
const JMP_FLAGS = {JMP: 0, JMPZ: 0,  JMPC: 0, JMPN: 0, return: 0, call: 0 };


function normalizeArrayIndex(st, useAddrRelProp = false) {
    if (typeof st.ind !== typeof st.indRR) {
        st.ind = st.ind ?? 0;
        st.indRR = st.indRR ?? 0;
    }
    delete st._fk;
    if (useAddrRelProp !== false && typeof st.useAddrRel !== 'undefined') {
        if (st.useAddrRel) st[useAddrRelProp] = 1;
        delete st.useAddrRel;
    }
}

function applyAddrRel(prefix, data) {
    let _jmp = {...data};
    if (_jmp.useAddrRel) {
        _jmp.ind = _jmp.ind ?? 0;
        _jmp.indRR = _jmp.indRR ?? 0;
        _jmp[`${prefix}UseAddrRel`] = 1;
        delete _jmp.useAddrRel;
    }
    _jmp[`${prefix}Addr`] = _jmp.offset ?? 0;
    delete _jmp.offset;
    _jmp[`${prefix}AddrLabel`] = _jmp.offsetLabel ?? '';
    delete _jmp.offsetLabel;
    return _jmp;
}

function applyCondConst(jmp, cond) {
    return { condConst: { type: '@final', value:  {type: '-' , values: [{type: 'CONSTL' , value: jmp.condConst}, cond]}}};
}
function setLine(dst, first) {
    dst.line = first.first_line;
}
%}

%start allStatments

%% /* language grammar */

allStatments
    : statmentList EOF
        {
            $$ = $1;
            return $$;
        }
    ;

statmentList
    : statmentList statment
        {
            if ($2) $1.push($2);
            $$ = $1;
        }
    | statment
        {
            if ($1) {
                $$ = [$1];
            } else {
                $$=[];
            }
        }
    ;

statment
    : step
        {
            $$ = $1;
        }
    | LABEL
        {
            $$ = {type: "label", identifier: $1};
            setLine($$, @1)

        }
    | varDef
        {
            $$ = $1;
        }
    | constDef
        {
            $$ = $1;
        }
    | include
        {
            $$ = $1;
        }
    | command
        {
            $$ = $1;
        }
    | LF
        {
            $$ = null;
        }
    ;

step
    : assignment ':' opList LF
        {
            $$ = {type: "step", assignment: $1, ops: $3};
            setLine($$, @1)
        }
    | assignment LF
        {
            $$ = {type: "step", assignment: $1, ops: []};
            setLine($$, @1)
        }
    | ':' opList  LF
        {
            $$ = {type: "step", assignment: null, ops: $2}
            setLine($$, @1)
        }
    ;

varDef
    :  VAR scope IDENTIFIER
        {
            $$ = {type: "var", scope: $2, name: $3, count: 1 }
        }
    |  VAR scope IDENTIFIER '[' NUMBER ']'
        {
            $$ = {type: "var", scope: $2, name: $3, count: $5 }
        }
    |  VAR scope IDENTIFIER '[' CONSTID ']'
        {
            $$ = {type: "var", scope: $2, name: $3, count: $5 }
        }
    ;

constDef
    : 'CONST' CONSTID '=' nexpr %prec '='
        {            
            // yy.parser.defineConstant($2, 'CONST', $4, @1.first_line);
            $$ = {type: "constdef", name: $2, value: $4}
            setLine($$, @1);
        }
    | 'CONSTL' CONSTID '=' nexpr %prec '='
        {
            // yy.parser.defineConstant($2, 'CONSTL', $4, @1.first_line);
            $$ = {type: "constldef", name: $2, value: $4}
            setLine($$, @1);
        }
    ;

command
    : COMMAND
        {
            $$ = {type: "command", cmd: $1}
        }
    ;

scope
    : GLOBAL
    | CTX
    ;

include
    : INCLUDE STRING
        {
            $$ = {type: "include", file: $2}
            setLine($$, @1)
        }
    ;

nexpr
    : NUMBER
        {
            $$ = {type: 'CONSTL' , value: $1}
        }
    | NUMBERL
        {
            $$ = {type: 'CONSTL' , value: $1}
        }
    | CONSTID
        {
            $$ = {type: 'CONSTID' , identifier: $1}
        }
    | CONSTID '??' nexpr
        {
            $$ = {type: $2, values: [$3] , identifier: $1}
        }
    | nexpr '+' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '-' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '*' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '**' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '%' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '/' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | '-' nexpr
        {
            $$ = {type: $1, values: [$2]}
        }
    | nexpr '<<' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '>>' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '|' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '&' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '^' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '<' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '>' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '<=' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '>=' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '==' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '!=' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '&&' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | nexpr '||' nexpr
        {
            $$ = {type: $2, values: [$1, $3]}
        }
    | '!' nexpr
        {
            $$ = {type: $1, values: [$2]}
        }
    | nexpr '?' nexpr ':' nexpr
        {
            $$ = {type: $2, values: [$1, $3, $5]}
        }
    | '(' nexpr ')'
        {
            $$ = $2
        }
    ;

assignment
    : inRegsSum '=>' destinationsList
        {
            $$ = {in: $1, out: $3}
        }
    | inRegsSum
        {
            $$ = {in: $1, out: []}
        }
    ;

inRegsSum
    : inRegsSum '+' inRegP
        {
            $$ = {type: 'add', values: [$1, $3]}
        }
    | inRegsSum '-' inRegP
        {
            $$ = {type: 'sub', values: [$1, $3]}
        }
    | '-' inRegP
        {
            $$ = {type: 'neg', values: [$2]}
        }
    | inRegP
        {
            $$ = $1
        }
    ;

inRegP
    : inRegP '*' inReg
        {
            $$ = {type: 'mul', values: [$1, $3]}
        }
    | inReg
        {
            $$ = $1
        }
    ;

inReg
    : TAG
        {
            $$ = {type: 'TAG' , tag: $1 }
        }
    | TAG_0
        {
            $$ = {type: 'TAG_0' , tag: $1 }
        }
    | TAG_BYTE
        {
            $$ = {type: 'TAG_BYTE' , tag: $1 }
        }
    | reg
        {
            $$ = {type: 'REG' , reg: $1}
        }
    | mem_addr
        {
            $$ = {type: 'F_MLOAD', addr: $1}
            normalizeArrayIndex($$.addr, 'memUseAddrRel');
        }
    | counter
        {
            $$ = {type: 'COUNTER', counter: $1}
        }

    | NUMBER '**' NUMBER
        {
            $$ = {type: "exp", values: [$1, $3]}
        }
    | NUMBERL '**' NUMBERL
        {
            $$ = {type: "expl", values: [$1, $3]}
        }
    | NUMBER
        {
            $$ = {type: 'CONST' , const: $1}
        }
    | NUMBERL
        {
            $$ = {type: 'CONSTL' , const: $1}
        }
    | CONSTID
        {
            $$ = {type: 'CONSTID' , identifier: $1}
        }
    | REFERENCE
        {
            $$ = {type: 'reference', identifier: $1}
        }
    ;

destinationsList
    : destinationsList ',' reg
        {
            $1.push({type: 'REG', reg:$3})
        }
    | destinationsList ',' mem_addr
        {
            normalizeArrayIndex($3);
            $1.push({type: 'MSTORE', addr:{...$3, assumeFree: 0}})
        }
    | mem_addr
        {
            normalizeArrayIndex($1);
            $$ = [{type: 'MSTORE', addr:{...$1, assumeFree: 0}}]
        }
    | reg
        {
            $$ = [{type: 'REG', reg:$1}]
        }
    ;

opList
    : opList ',' op
        {
            $1.push($3);
            $$ = $1
        }
    | op
        {
            $$ = [$1]
        }
    ;

op
    : MLOAD '(' addr ')'
        {
            normalizeArrayIndex($3, 'memUseAddrRel');            
            $$ = { offset: 0, ...$3, mOp: 1, mWR: 0, assumeFree: 0 };
        }
    | F_MLOAD '(' addr ')'
        {
            normalizeArrayIndex($3, 'memUseAddrRel');
            $$ = { offset: 0, ...$3, mOp: 1, mWR: 0, assumeFree: 1 };
        }
    | MSTORE '(' addr ')'
        {
            normalizeArrayIndex($3, 'memUseAddrRel');
            $$ = { offset: 0, ...$3, mOp: 1, mWR: 1, assumeFree: 0 };
        }
    | JMP '(' jmp_addr ')'
        {
            $$ = {...JMP_FLAGS, JMP: 1, ...applyAddrRel('jmp', $3) }
        }
    | jmpNotCond '(' jmp_addr ')'
        {
            $$ = { ...JMP_FLAGS,  ...$1, ...applyAddrRel('else', $3), jmpAddr: 0, jmpAddrLabel: 'next' }
        }
    | jmpNotCond '(' jmp_addr ',' jmp_addr ')'
        {
            {
                let _else = applyAddrRel('else', $3);
                let _jmp = applyAddrRel('jmp', $5);

                if (_jmp.jmpUseAddrRel && _else.elseUseAddrRel && 
                    (!lodash.isEqual(_jmp.ind, _else.ind) || !lodash.isEqual(_jmp.indRR, _else.indRR))) {
                        this.compiler._error(`Diferent relative address between jmp and else addresses`);
                }
                $$ = { ...JMP_FLAGS, ...$1, ..._jmp, ..._else }            
            }
        }
    | jmpCond '(' jmp_addr ')'
        {   
            $$ = {...JMP_FLAGS, ...$1, ...applyAddrRel('jmp', $3), elseAddr: 0, elseAddrLabel: 'next' };
        }
    | jmpCond '(' jmp_addr ',' jmp_addr ')'
        {
            {
                let _else = applyAddrRel('else', $5);
                let _jmp = applyAddrRel('jmp', $3);

                if (_jmp.jmpUseAddrRel && _else.elseUseAddrRel && 
                    (!lodash.isEqual(_jmp.ind, _else.ind) || !lodash.isEqual(_jmp.indRR, _else.indRR))) {
                        this.compiler._error(`Diferent relative address between jmp and else addresses`);
                }
                $$ = {...JMP_FLAGS, ...$1, ..._jmp, ..._else };
            }
        }

    // condConst conditionals

    | jmpNotCondConst '(' nexpr ',' jmp_addr ')'
        {
            $$ = { ...JMP_FLAGS,  ...$1, ...applyAddrRel('else', $5), jmpAddr: 0, jmpAddrLabel: 'next', ...applyCondConst($1, $3)};
        }
    | jmpNotCondConst '(' nexpr ',' jmp_addr ',' jmp_addr ')'
        {
            {
                let _else = applyAddrRel('else', $5);
                let _jmp = applyAddrRel('jmp', $7);

                if (_jmp.jmpUseAddrRel && _else.elseUseAddrRel && 
                    (!lodash.isEqual(_jmp.ind, _else.ind) || !lodash.isEqual(_jmp.indRR, _else.indRR))) {
                        this.compiler._error(`Diferent relative address between jmp and else addresses`);
                }
                $$ = { ...JMP_FLAGS, ...$1, ..._jmp, ..._else, ...applyCondConst($1, $3)}

            }
        }
    | jmpCondConst '(' nexpr ',' jmp_addr ')'
        {   
            $$ = {...JMP_FLAGS, ...$1, ...applyAddrRel('jmp', $5), elseAddr: 0, elseAddrLabel: 'next', 
                  ...applyCondConst($1, $3)};
        }
    | jmpCondConst '(' nexpr ',' jmp_addr ',' jmp_addr ')'
        {
            {
                let _else = applyAddrRel('else', $7);
                let _jmp = applyAddrRel('jmp', $5);

                if (_jmp.jmpUseAddrRel && _else.elseUseAddrRel && 
                    (!lodash.isEqual(_jmp.ind, _else.ind) || !lodash.isEqual(_jmp.indRR, _else.indRR))) {
                        this.compiler._error(`Diferent relative address between jmp and else addresses`);
                }
                $$ = {...JMP_FLAGS, ...$1, ..._jmp, ..._else, ...applyCondConst($1, $3)}
            }
        }

    | CALL '(' jmp_addr ')'
        {
            $$ = {...JMP_FLAGS, JMP: 1, call: 1, ...applyAddrRel('jmp', $3) }
        }

    | RETURN
        {
            $$ = {JMP: 0, JMPC: 0, JMPN: 0,  call: 0, return: 1}
        }
    | ASSERT
        {
            $$ = {assert: 1}
        }
    | REPEAT '(' RCX ')'
        {
            $$ = { repeat: 1 }
        }
    ;

jmpCond
    : JMPN    { $$ = { JMPN: 1, free0IsByte: 0 } }
    | JMPC    { $$ = { JMPC: 1 } }
    | JMPZ    { $$ = { JMPZ: 1 } }
    | CALL_Z  { $$ = { JMPZ: 1, call: 1 } }
    | CALL_N  { $$ = { JMPN: 1, call: 1, free0IsByte: 0 } }
    | CALL_C  { $$ = { JMPC: 1, call: 1 } }
    ;

jmpCondConst
    : JMP_EQ  { $$ = { JMPZ: 1, condConst:  0 } }
    | JMP_LT  { $$ = { JMPN: 1, condConst:  0, free0IsByte: 0 } }
    | JMP_LE  { $$ = { JMPN: 1, condConst: -1, free0IsByte: 0 } }
    | CALL_EQ { $$ = { JMPZ: 1, condConst:  0, call: 1 } }
    | CALL_LT { $$ = { JMPN: 1, condConst:  0, call: 1, free0IsByte: 0 } }
    | CALL_LE { $$ = { JMPN: 1, condConst: -1, call: 1, free0IsByte: 0 } }
    ;

jmpNotCond
    : JMPNC   { $$ = { JMPC: 1 } }
    | JMPNZ   { $$ = { JMPZ: 1 } }
    | CALL_NC { $$ = { JMPC: 1, call: 1 } }
    | CALL_NZ { $$ = { JMPZ: 1, call: 1 } }
    ;

jmpNotCondConst
    : JMP_NE  { $$ = { JMPZ: 1, condConst:  0 } }
    | JMP_GT  { $$ = { JMPN: 1, condConst: -1, free0IsByte: 0 } }
    | JMP_GE  { $$ = { JMPN: 1, condConst:  0, free0IsByte: 0 } }
    | CALL_NE { $$ = { JMPZ: 1, condConst:  0, call: 1 } }
    | CALL_GT { $$ = { JMPN: 1, condConst: -1, call: 1, free0IsByte: 0 } }
    | CALL_GE { $$ = { JMPN: 1, condConst:  0, call: 1, free0IsByte: 0 } }
    ;


reg
    : A
    | B
    | C
    | D
    | E
    | SR
    | CTX
    | SP
    | PC
    | RR
    | zkPC
    | STEP
    | ROTL_C
    | RCX
    ;

addr

    : SP
        {
            $$ = { isStack: 1, isMem:0, incStack:0, offset: 0, useCTX: 1}
        }
    | SP '+' NUMBER
        {
            $$ = { isStack: 1, isMem:0, incStack: 0, offset: $3, useCTX: 1}
        }
    | SP '-' NUMBER
        {
            $$ = { isStack: 1, isMem:0, incStack: 0, offset: -$3, useCTX: 1}
        }
    | SP '++'
        {
            $$ = { isStack: 1, isMem:0, incStack: 1, offset: 0, useCTX: 1}
        }
    | SP '--'
        {
            $$ = { isStack: 1, isMem:0, incStack: -1, offset: 0, useCTX: 1}
        }
    | SYS ':' array_index
        {            
            $$ = { isStack: 0, isMem:0, incStack: 0, ...$3 }
            normalizeArrayIndex($$);
        }
    | MEM ':' array_index
        {
            $$ = { isStack: 0, isMem: 1, incStack: 0, useCTX: 1, ...$3 }
            normalizeArrayIndex($$);
        }
    | STACK ':' array_index
        {
            $$ = { isStack: 1, isMem: 0, incStack: 0, useCTX: 1, ...$3 }
            normalizeArrayIndex($$);
        }
    | SYS '[' array_index ']'
        {
            $$ = { isStack: 0, isMem:0, incStack: 0, ...$3 }
            normalizeArrayIndex($$);
        }
    | MEM '[' array_index ']'
        {
            $$ = { isStack: 0, isMem: 1, incStack: 0, useCTX: 1, ...$3 }
            normalizeArrayIndex($$);
        }
    | STACK '[' array_index ']'
        {
            $$ = { isStack: 1, isMem: 0, incStack: 0, useCTX: 1, ...$3 }
            normalizeArrayIndex($$);
        }
    | IDENTIFIER '[' array_index ']'
        {
            $$ = { offsetLabel: $1, ...$3 }
            normalizeArrayIndex($$);
        }
    | IDENTIFIER
        {
            $$ = { offsetLabel: $1, offset: 0 }
        }
    | IDENTIFIER '+' RR
        {
            $$ = { offsetLabel: $1, offset: 0, memUseAddrRel: 1, ind: 0, indRR: 1 }
        }
    | IDENTIFIER '+' E
        {
            $$ = { offsetLabel: $1, offset: 0, memUseAddrRel: 1, ind: 1, indRR: 0 }
        }
    | IDENTIFIER '+' NUMBER
        {
            $$ = { offsetLabel: $1, offset: $3 }
        }
    ;

jmp_addr

    : array_index
        {
            $$ = { ...$1 }
        }            

    | IDENTIFIER
        {
            $$ = { offsetLabel: $1 }
        }            

    | REFERENCE
        {   
            $$ = { offsetLabel: $1 }
        }            

    | REFERENCE '+' array_index
        {   
            {
                let _aindex = { ...$3 };
                delete _aindex._fk;
                $$ = { offsetLabel: $1, ..._aindex }
            }
        }            

    | REFERENCE '-' array_index
        {
            {
                let _aindex = { ...$3 };
                _aindex[_aindex._fk] = { type: 'neg', values: [_aindex[_aindex._fk]]};
                delete _aindex._fk;
                $$ = { offsetLabel: $1, ..._aindex }
            }
        }            
    ;


short_const_value

    : NUMBER
        {
            $$ = {type: 'CONST' , const: Number($1) }
        }
    | CONSTID
        {
            $$ = {type: 'CONSTID' , identifier: $1 }
        }
    ;

array_index
    : array_index '+' array_index_item
        {
            Object.keys($3).forEach(k => {
                if (!k.startsWith('_') && (k !== 'useAddrRel' || !lodash.isEqual($1[k], $3[k]))) {
                    if ($1[k] && $1[k].const !== 0) {
                        this.compiler._error(`Property ${k} already used`);
                    }
                    $1[k] = $3[k];
                }
            });
            $$ = $1;
        }
    | array_index '-' array_index_item
        {
            Object.keys($3).forEach(k => {
                if (!k.startsWith('_') && (k !== 'useAddrRel' || !lodash.isEqual($1[k], $3[k]))) {
                    if ($1[k] && $1[k].const !== 0) {
                        this.compiler._error(`Property ${k} already used`);
                    }
                    if (k === $3._fk) {
                        $1[k] = {type: 'neg', values: [$3[k]]};
                    } else {
                        $1[k] = $3[k];
                    }
                }
            });
            $$ = $1;
        }
    | array_index_item
        {
            $$ = $1
        }
    ;

array_index_item

    : short_const_value
        {
            $$ = { _fk: 'offset', offset: $1 }
        }
    | E
        {
            $$ = { _fk: 'ind', useAddrRel: 1, ind: 1 }
        }
    | RR
        {
            $$ = { _fk: 'indRR', useAddrRel: 1, indRR: 1 }            
        }
    | short_const_value '*' E
        {
            $$ = { _fk: 'ind', useAddrRel: 1, ind: $1 }
        }
    | short_const_value '*' RR
        {
            $$ = { _fk: 'indRR', useAddrRel: 1, indRR: $1 }
        }
    | E  '*' short_const_value 
        {
            $$ = { _fk: 'ind', useAddrRel: 1, ind: $3 }
        }
    | RR '*' short_const_value 
        {
            $$ = { _fk: 'indRR', useAddrRel: 1, indRR: $3 }
        }
    ;


mem_addr
    : IDENTIFIER    %prec EMPTY
        {
            $$ = { offsetLabel: $1 }
        }
    | SYS '[' array_index ']'
        {
            $$ = { isStack: 0, isMem:0, incStack: 0, ...$3 }
            delete $$._fk;
        }
    | MEM '[' array_index ']'
        {
            $$ = { isStack: 0, isMem: 1, incStack: 0, useCTX: 1, ...$3 }
            delete $$._fk;
        }
    | STACK '[' array_index ']'
        {
            $$ = { isStack: 1, isMem: 0, incStack: 0, useCTX: 1, ...$3 }
            delete $$._fk;
        }
    | IDENTIFIER '[' array_index ']'
        {
            $$ = { offsetLabel: $1, ...$3 }
            delete $$._fk;
        }
    ;