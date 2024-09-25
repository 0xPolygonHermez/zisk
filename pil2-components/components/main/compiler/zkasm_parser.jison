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
GAS                     { return 'GAS'; }
zkPC                    { return 'zkPC'; }
RR                      { return 'RR'; }
STEP                    { return 'STEP'; }
ROTL_C                  { return 'ROTL_C'; }
HASHPOS                 { return 'HASHPOS'; }
RID                     { return 'RID'; }
F_MLOAD                 { return 'F_MLOAD' }
MLOAD                   { return 'MLOAD' }
MSTORE                  { return 'MSTORE' }
HASHKLEN                { return 'HASHKLEN' }
HASHKDIGEST             { return 'HASHKDIGEST' }
F_HASHK((1[0-9])|(2[0-9])|(3[0-2])|[1-9])  { yytext = yytext.slice(7); return 'F_HASHKn' }
HASHK((1[0-9])|(2[0-9])|(3[0-2])|[1-9])  { yytext = yytext.slice(5); return 'HASHKn' }
F_HASHK                 { return 'F_HASHK' }
HASHK                   { return 'HASHK' }
HASHSLEN                { return 'HASHSLEN' }
HASHSDIGEST             { return 'HASHSDIGEST' }
F_HASHS((1[0-9])|(2[0-9])|(3[0-2])|[1-9])  { yytext = yytext.slice(7); return 'F_HASHSn' }
HASHS((1[0-9])|(2[0-9])|(3[0-2])|[1-9])  { yytext = yytext.slice(5); return 'HASHSn' }
F_HASHS                   { return 'F_HASHS' }
HASHS                   { return 'HASHS' }
HASHPLEN                { return 'HASHPLEN' }
HASHPDIGEST             { return 'HASHPDIGEST' }
F_HASHP((1[0-9])|(2[0-9])|(3[0-2])|[1-9])  { yytext = yytext.slice(7); return 'F_HASHPn' }
HASHP((1[0-9])|(2[0-9])|(3[0-2])|[1-9])  { yytext = yytext.slice(5); return 'HASHPn' }
F_HASHP                 { return 'F_HASHP' }
HASHP                   { return 'HASHP' }
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
SLOAD                   { return 'SLOAD' }
SSTORE                  { return 'SSTORE' }
ARITH                   { return 'ARITH' }
ARITH_ECADD_DIFFERENT   { return 'ARITH_ECADD_DIFFERENT' }
ARITH_ECADD_SAME        { return 'ARITH_ECADD_SAME' }
ARITH_BN254_MULFP2      { return 'ARITH_BN254_MULFP2' }
ARITH_BN254_ADDFP2      { return 'ARITH_BN254_ADDFP2' }
ARITH_BN254_SUBFP2      { return 'ARITH_BN254_SUBFP2' }
ARITH_MOD               { return 'ARITH_MOD' }
ADD                     { return 'ADD' }
SUB                     { return 'SUB' }
LT                      { return 'LT' }
SLT                     { return 'SLT' }
EQ                      { return 'EQ' }
AND                     { return 'AND' }
OR                      { return 'OR' }
XOR                     { return 'XOR' }
LT4                     { return 'LT4' }
CNT_ARITH               { return 'CNT_ARITH' }
CNT_BINARY              { return 'CNT_BINARY' }
CNT_KECCAK_F            { return 'CNT_KECCAK_F' }
CNT_SHA256_F            { return 'CNT_SHA256_F' }
CNT_MEM_ALIGN           { return 'CNT_MEM_ALIGN' }
CNT_PADDING_PG          { return 'CNT_PADDING_PG' }
CNT_POSEIDON_G          { return 'CNT_POSEIDON_G' }
MEM_ALIGN_WR8           { return 'MEM_ALIGN_WR8' }
MEM_ALIGN_RD            { return 'MEM_ALIGN_RD' }
MEM_ALIGN_WR            { return 'MEM_ALIGN_WR' }
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
SAVE                    { return 'SAVE' }
RESTORE                 { return 'RESTORE' }
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

saveRegsList
    : saveRegsList ',' saveReg
        {
            $1.push($3)
        }
    | saveReg
        {
            $$ = [$1]
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
    | F_HASHK '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashP = 0;
            $$.hashK = 1;
            $$.hashBytesInD = 1;
            $$.hashBytes = 0;
            $$.assumeFree = 1;
        }
    | F_HASHKn '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashP = 0;
            $$.hashK = 1;
            $$.hashBytesInD = 0;
            $$.hashBytes = Number($1);
            $$.assumeFree = 1;
        }
    | HASHK '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashP = 0;
            $$.hashK = 1;
            $$.hashBytesInD = 1;
            $$.hashBytes = 0;
            $$.assumeFree = 0;
        }
    | HASHKn '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashP = 0;
            $$.hashK = 1;
            $$.hashBytesInD = 0;
            $$.hashBytes = Number($1);
            $$.assumeFree = 0;
        }
    | HASHKLEN '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashP = 0;
            $$.hashK = 0;
            $$.hashKLen = 1;
        }
    | HASHKDIGEST '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashP = 0;
            $$.hashK = 0;
            $$.hashKDigest = 1;
        }
    | F_HASHS '(' hashId ')'
        {
            $$ = $3;
            $$.hashP = 0;
            $$.hashK = 0;
            $$.hashS = 1;
            $$.hashBytesInD = 1;
            $$.hashBytes = 0;
            $$.assumeFree = 1;
        }
    | F_HASHSn '(' hashId ')'
        {
            $$ = $3;
            $$.hashP = 0;
            $$.hashK = 0;
            $$.hashS = 1;
            $$.hashBytesInD = 0;
            $$.hashBytes = Number($1);
            $$.assumeFree = 1;
        }
    | HASHS '(' hashId ')'
        {
            $$ = $3;
            $$.hashP = 0;
            $$.hashK = 0;
            $$.hashS = 1;
            $$.hashBytesInD = 1;
            $$.hashBytes = 0;
            $$.assumeFree = 0;
        }
    | HASHSn '(' hashId ')'
        {
            $$ = $3;
            $$.hashP = 0;
            $$.hashK = 0;
            $$.hashS = 1;
            $$.hashBytesInD = 0;
            $$.hashBytes = Number($1);
            $$.assumeFree = 0;
        }
    | HASHSLEN '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashP = 0;
            $$.hashK = 0;
            $$.hashSLen = 1;
        }
    | HASHSDIGEST '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashP = 0;
            $$.hashK = 0;
            $$.hashSDigest = 1;
        }
    | F_HASHP '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashK = 0;
            $$.hashP = 1;
            $$.hashBytesInD = 1;
            $$.hashBytes = 0;
            $$.assumeFree = 1;
        }
    | F_HASHPn '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashK = 0;
            $$.hashP = 1;
            $$.hashBytesInD = 0;
            $$.hashBytes = Number($1);
            $$.assumeFree = 1;
        }
    | HASHP '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashK = 0;
            $$.hashP = 1;
            $$.hashBytesInD = 1;
            $$.hashBytes = 0;
            $$.assumeFree = 0;
        }
    | HASHPn '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashK = 0;
            $$.hashP = 1;
            $$.hashBytesInD = 0;
            $$.hashBytes = Number($1);
            $$.assumeFree = 0;
        }
    | HASHPLEN '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashP = 0;
            $$.hashK = 0;
            $$.hashPLen = 1;
        }
    | HASHPDIGEST '(' hashId ')'
        {
            $$ = $3;
            $$.hashS = 0;
            $$.hashP = 0;
            $$.hashK = 0;
            $$.hashPDigest = 1;
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
    | SLOAD
        {
            $$ = {sRD: 1}
        }
    | SSTORE
        {
            $$ = {sWR: 1}
        }
    | ARITH
        {
            $$ = { arith: 1, arithSame12: 0, arithUseE: 0, arithEquation: 1 }
        }
    | ARITH_ECADD_DIFFERENT
        {
            $$ = { arith: 1, arithSame12: 0, arithUseE: 1, arithEquation: 2 }
        }
    | ARITH_ECADD_SAME
        {
            $$ = { arith: 1, arithSame12: 1, arithUseE: 1, arithEquation: 3 }
        }
    | ARITH_BN254_MULFP2
        {
            $$ = { arith: 1, arithSame12: 0, arithUseE: 1, arithEquation: 4 }
        }
    | ARITH_BN254_ADDFP2
        {
            $$ = { arith: 1, arithSame12: 0, arithUseE: 1, arithEquation: 5 }
        }
    | ARITH_BN254_SUBFP2
        {
            $$ = { arith: 1, arithSame12: 0, arithUseE: 1, arithEquation: 6 }
        }
    | ARITH_MOD
        {
            $$ = { arith: 1, arithSame12: 0, arithUseE: 0, arithEquation: 7 }
        }
    | ADD
        {
            $$ = { bin: 1, binOpcode: 0}
        }
    | SUB
        {
            $$ = { bin: 1, binOpcode: 1}
        }
    | LT
        {
            $$ = { bin: 1, binOpcode: 2}
        }
    | SLT
        {
            $$ = { bin: 1, binOpcode: 3}
        }
    | EQ
        {
            $$ = { bin: 1, binOpcode: 4}
        }
    | AND
        {
            $$ = { bin: 1, binOpcode: 5}
        }
    | OR
        {
            $$ = { bin: 1, binOpcode: 6}
        }
    | XOR
        {
            $$ = { bin: 1, binOpcode: 7}
        }
    | LT4
        {
            $$ = { bin: 1, binOpcode: 8}
        }
    | MEM_ALIGN_RD
        {
            $$ = { memAlignRD: 1, memAlignWR: 0, memAlignWR8: 0}
        }
    | MEM_ALIGN_WR
        {
            $$ = { memAlignRD: 0, memAlignWR: 1, memAlignWR8: 0}
        }
    | MEM_ALIGN_WR8
        {
            $$ = { memAlignRD: 0, memAlignWR: 0, memAlignWR8: 1}
        }
    | REPEAT '(' RCX ')'
        {
            $$ = { repeat: 1 }
        }
    | SAVE '(' saveRegsList ')'
        {
            $$ = { save: 1, restore: 0, regs: $3 }
        }
    | RESTORE '(' saveRegsList ')'
        {
            $$ = { save: 0, restore: 1, regs: $3 }
        }
    | RESTORE
        {
            $$ = { save: 0, restore: 1, regs: false }
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


counter
    : CNT_ARITH         { $$ = 'cntArith' }
    | CNT_BINARY        { $$ = 'cntBinary' }
    | CNT_KECCAK_F      { $$ = 'cntKeccakF' }
    | CNT_SHA256_F      { $$ = 'cntSha256F' }
    | CNT_MEM_ALIGN     { $$ = 'cntMemAlign' }
    | CNT_PADDING_PG    { $$ = 'cntPaddingPG' }
    | CNT_POSEIDON_G    { $$ = 'cntPoseidonG' }
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
    | GAS
    | RR
    | zkPC
    | STEP
    | HASHPOS
    | ROTL_C
    | RCX
    | RID
    ;

saveReg
    : A
    | B
    | C
    | D
    | E
    | SR
    | SP
    | PC
    | RR
    | HASHPOS
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


hashId
    : E
        {
            $$ = { hashOffset: 0 }
        }
    | E '+' NUMBER
        {
            $$ = { hashOffset:$3 }
        }
    | E '-' NUMBER
        {
            $$ = { hashOffset: -$3 }
        }
    ;