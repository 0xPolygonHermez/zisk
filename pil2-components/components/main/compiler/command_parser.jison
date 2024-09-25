/* lexical grammar */
%lex
%%
(0x[0-9A-Fa-f][0-9A-Fa-f_]*)|([0-9][0-9_]*)          { yytext = BigInt(yytext.replace(/\_/g, "")); return 'NUMBER'; }
[ \t\r\n]+                  { /* console.log("Empty spaces"); */ }
A                       { return 'A'; }
B                       { return 'B'; }
C                       { return 'C'; }
D                       { return 'D'; }
E                       { return 'E'; }
SR                      { return 'SR'; }
CTX                     { return 'CTX'; }
SP                      { return 'SP'; }
PC                      { return 'PC'; }
GAS                     { return 'GAS'; }
zkPC                    { return 'zkPC'; }
RR                      { return 'RR'; }
STEP                    { return 'STEP'; }
CNT_ARITH               { return 'CNT_ARITH' }
CNT_BINARY              { return 'CNT_BINARY' }
CNT_KECCAK_F            { return 'CNT_KECCAK_F' }
CNT_SHA256_F            { return 'CNT_SHA256_F' }
CNT_MEM_ALIGN           { return 'CNT_MEM_ALIGN' }
CNT_PADDING_PG          { return 'CNT_PADDING_PG' }
CNT_POSEIDON_G          { return 'CNT_POSEIDON_G' }
HASHPOS                 { return 'HASHPOS'; }
RCX                     { return 'RCX'; }
RID                     { return 'RID'; }
NRID                    { return 'NRID'; }
var                     { return 'VAR'; }
[a-zA-Z_][a-zA-Z$_0-9]*  { return 'IDENTIFIER'; }
\(                      { return '('}
\)                      { return ')'}
\+                      { return '+'}
\,                      { return ','}
\-                      { return '-'}
\*\*                    { return 'EXP'}
\*                      { return '*'}
\/                      { return '/'}
\%                      { return '%'}
\<\<                    { return 'SHL'}
\>\>                    { return 'SHR'}
\<\=                    { return 'LE'}
\>\=                    { return 'GE'}
\&\&                    { return 'L_AND'}
\|\|                    { return 'L_OR'}
\=\=                    { return 'EQ'}
\!\=                    { return 'NE'}
\=                      { return '=' }
\&                      { return '&'}
\~                      { return '~'}
\|                      { return '|'}
\^                      { return '^'}
\<                      { return 'LT'}
\>                      { return 'GT'}
\!                      { return '!'}
\?                      { return '?'}
\:                      { return ':'}
\.\.                    { return RANGE_DOTS }
\.                      { return '.'}
\[                      { return '[' }
\]                      { return ']' }
<<EOF>>                 { return 'EOF'; }
.                       { /* console.log("INVALID: " + yytext); */ return 'INVALID'; }

/lex

%left EMPTY
%left ','
%right '='
%left '+' '-'
%left '*' '/' '%'
%left SHL SHR
%left '&' '|'
%right '!' UMINUS UPLUS
%{
%}

%start tag

%% /* language grammar */

tag
    : expression EOF
        {
            // console.log($1);
            $$ = $1;
            return $$;
        }
    ;


expression
    : e5 %prec EMPTY
        {
            $$ = $1;
        }
    ;

e5
    : leftExpression '=' e5
        {
            $$ = { op: "setVar", values: [$1, $3] };
        }
    | e4 %prec EMPTY
        {
            $$ = $1;
        }
    ;

e4
    : e4 '+' e3
        {
            $$ = { op: "add", values: [$1, $3] };
        }
    | e4 '-' e3
        {
            $$ = { op: "sub", values: [$1, $3] };
        }
    | e3 %prec EMPTY
        {
            $$ = $1;
        }
    ;

e3
    : e3 '*' e2
        {
            $$ = { op: "mul", values: [$1, $3] };
        }
    | e3 '/' e2
        {
            $$ = { op: "div", values: [$1, $3] };
        }
    | e3 '%' e2
        {
            $$ = { op: "mod", values: [$1, $3] };
        }
    | e3 '&' e2
        {
            $$ = { op: "bitand", values: [$1, $3] };
        }
    | e3 '|' e2
        {
            $$ = { op: "bitor", values: [$1, $3] };
        }
    | e3 '^' e2
        {
            $$ = { op: "bitxor", values: [$1, $3] };
        }
    | e3 SHL e2
        {
            $$ = { op: "shl", values: [$1, $3] };
        }
    | e3 SHR e2
        {
            $$ = { op: "shr", values: [$1, $3] };
        }
    | e3 L_OR e2
        {
            $$ = { op: "or", values: [$1, $3] };
        }
    | e3 L_AND e2
        {
            $$ = { op: "and", values: [$1, $3] };
        }
    | e3 EXP e2
        {
            $$ = { op: "exp", values: [$1, $3] };
        }
    | e3 EQ e2
        {
            $$ = { op: "eq", values: [$1, $3] };
        }
    | e3 NE e2
        {
            $$ = { op: "ne", values: [$1, $3] };
        }
    | e3 LT e2
        {
            $$ = { op: "lt", values: [$1, $3] };
        }
    | e3 LE e2
        {
            $$ = { op: "le", values: [$1, $3] };
        }
    | e3 GT e2
        {
            $$ = { op: "gt", values: [$1, $3] };
        }
    | e3 GE e2
        {
            $$ = { op: "ge", values: [$1, $3] };
        }
    | e3 '?' e2 ':' e2
        {
            $$ = { op: "if", values: [$1, $3, $5] };
        }
    | e2 %prec EMPTY
        {
            $$ = $1;
        }
    ;

e2
    : '+' e2 %prec UPLUS
        {
            $$ = $2;
        }
    | '-' e2 %prec UMINUS
        {
            $$ = { op: "neg", values: [$2] };
        }
    | '~' e2 %prec NOT
        {
            $$ = { op: "bitnot", values: [$2] };
        }
    | '!' e2 %prec NOT
        {
            $$ = { op: "not", values: [$2] };
        }
    | e1 %prec EMPTY
        {
            $$ = $1;
        }
    ;


e1
    : functionCall
        {
            $$ = $1;
        }
    | e0
        {
            $$ = $1
        }
    ;

e0
    :   leftExpression
        {
            $$ = $1
        }
    | NUMBER
        {
            $$ = {op: "number", num: $1 }
        }
    | reg
        {
            $$ = {op: "getReg", regName: $1}
        }
    | counter
        {
            $$ = {op: "getReg", regName: $1}
        }
    | '(' expression ')'
        {
            $$ = $2;
        }
    | IDENTIFIER '.' IDENTIFIER
        {
            $$ = {op: "getData", module: $1, offset: $3, arrayOffset: 0}
        }
    | IDENTIFIER '.' IDENTIFIER '[' expression ']'
        {
            $$ = {op: "getData", module: $1, offset: $3, arrayOffset: $5}
        }
    ;

leftExpression
    : VAR IDENTIFIER
        {
            $$ = {op: "declareVar", varName: $2}
        }
    | IDENTIFIER
        {
            $$ = {op: "getVar", varName: $1}
        }
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
    | zkPC
    | RR
    | STEP
    | HASHPOS
    | RCX
    | RID
    | NRID
    ;

counter
    : CNT_ARITH
    | CNT_BINARY
    | CNT_KECCAK_F
    | CNT_SHA256_F
    | CNT_MEM_ALIGN
    | CNT_PADDING_PG
    | CNT_POSEIDON_G
    ;

functionCall
    : IDENTIFIER '(' expressionList ')'
        {
            $$ = {op: "functionCall", funcName: $1, params: $3}
        }
    | IDENTIFIER '(' ')'
        {
            $$ = {op: "functionCall", funcName: $1, params: []}
        }
    ;

expressionList
    : expressionList ',' expression
        {
            $1.push($3);
        }
    | expression %prec ','
        {
            $$ = [$1];
        }
    ;

