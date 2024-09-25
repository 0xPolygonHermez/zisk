/* lexical grammar */
%lex
%%
(0x[0-9A-Fa-f][0-9A-Fa-f_]*)|([0-9][0-9_]*)          { yytext = Number(yytext.replace(/\_/g, "")); return 'NUMBER'; }
[ \t\r\n]+                  { /* console.log("Empty spaces"); */ }
HASH_LEFT               { return 'HASH_LEFT'; }
HASH_RIGHT              { return 'HASH_RIGHT'; }
OLD_ROOT                { return 'OLD_ROOT'; }
NEW_ROOT                { return 'NEW_ROOT'; }
VALUE_LOW               { return 'VALUE_LOW'; }
VALUE_HIGH              { return 'VALUE_HIGH'; }
SIBLING_VALUE_HASH      { return 'SIBLING_VALUE_HASH'; }
FREE                    { return 'FREE'; }
RKEY                    { return 'RKEY'; }
SIBLING_RKEY            { return 'SIBLING_RKEY'; }
RKEY_BIT                { return 'RKEY_BIT'; }
LEVEL                   { return 'LEVEL'; }
ROTL_VH                 { return 'ROTL_VH'; }
PC                      { return 'PC'; }
var                     { return 'VAR'; }
[a-zA-Z_][a-zA-Z$_0-9\+]*  { return 'IDENTIFIER'; }
\(                      { return '('}
\)                      { return ')'}
\+                      { return '+'}
\,                      { return ','}
\-                      { return '-'}
\*                      { return '*'}
\/                      { return '/'}
\%                      { return '%'}
\=                      { return '=' }
<<EOF>>                 { return 'EOF'; }
.                       { /* console.log("INVALID: " + yytext); */ return 'INVALID'; }

/lex

%left EMPTY
%left ','
%right '='
%left '+' '-'
%left '*' '/' '%'
%right UMINUS UPLUS

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
    | '(' expression ')'
        {
            $$ = $2;
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
    : HASH_LEFT
    | HASH_RIGHT
    | OLD_ROOT
    | NEW_ROOT
    | VALUE_LOW
    | VALUE_HIGH
    | SIBLING_VALUE_HASH
    | RKEY
    | SIBLING_RKEY
    | RKEY_BIT
    | LEVEL
    | PC
    | ROTL_VH
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

