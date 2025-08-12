# Vitte — EBNF complète (v1.3)

program     = { item } ;
item        = fn_decl | struct_decl | union_decl | enum_decl | const_decl | static_decl | use_decl | mod_decl | type_alias | macro_decl ;

use_decl    = "use" path ( "as" ident )? ";" ;
mod_decl    = "mod" path ";" ;

type_alias  = "type" ident "=" ty ";" ;

fn_decl     = attrs? "fn" ident type_params? "(" params? ")" ret_ty? where_clause? block ;
type_params = "<" type_param { "," type_param } ">" ;
type_param  = ident ( ":" trait_bounds )? ;
trait_bounds= path { "+" path } ;
params      = param { "," param } ;
param       = ident ":" ty ( "=" expr )? ;

ret_ty      = "->" ty ;
where_clause= "where" where_item { "," where_item } ;
where_item  = ident ":" trait_bounds ;

struct_decl = attrs? "struct" ident "{" fields? "}" ;
fields      = field { "," field } ;
field       = ident ":" ty ;

union_decl  = attrs? "union" ident "{" fields? "}" ;

enum_decl   = attrs? "enum" ident "{" variants? "}" ;
variants    = variant { "," variant } ;
variant     = ident | ident "{" fields? "}" | ident "(" ty { "," ty } ")" ;

const_decl  = "const" ident ":" ty "=" expr ";" ;
static_decl = "static" "mut"? ident ":" ty "=" expr ";" ;

trait_decl  = "trait" ident type_params? "{" trait_items? "}" ;
impl_decl   = "impl" type_params? for_ty "{" impl_items? "}" ;
for_ty      = ty "for" path | path ;
trait_items = { fn_sig ";" | const_sig ";" | type_sig ";" } ;
impl_items  = { fn_decl | const_decl | type_alias } ;

block       = "{" { stmt } "}" ;
stmt        = let_stmt | expr_stmt | return_stmt | if_stmt | while_stmt | for_stmt | loop_stmt | match_stmt | defer_stmt ;
let_stmt    = "let" "mut"? pat ":"? ty? "=" expr ";" ;
expr_stmt   = expr ";" ;
return_stmt = "return" expr? ";" ;
defer_stmt  = "defer" block ;

if_stmt     = "if" expr block ( "else" ( block | if_stmt ) )? ;
while_stmt  = "while" expr block ;
for_stmt    = "for" pat "in" expr block ;
loop_stmt   = "loop" block ;
match_stmt  = "match" expr "{" match_arm { "," match_arm } "}" ;
match_arm   = pat guard? "=>" ( expr | block ) ;
guard       = "if" expr ;

pat         = "_" | ident | lit | tuple_pat | struct_pat | enum_pat ;
tuple_pat   = "(" pat { "," pat } ")" ;
struct_pat  = path "{" field_pat { "," field_pat } "}" ;
field_pat   = ident ":" pat | ident ;
enum_pat    = path "(" pat { "," pat } ")" ;

expr        = assign ;
assign      = logic_or { assign_op logic_or } ;
assign_op   = "=" | "+=" | "-=" | "*=" | "/=" ;
logic_or    = logic_and { "||" logic_and } ;
logic_and   = bit_or    { "&&" bit_or } ;
bit_or      = bit_xor   { "|"  bit_xor } ;
bit_xor     = bit_and   { "^"  bit_and } ;
bit_and     = equality  { "&"  equality } ;
equality    = relation  { ( "==" | "!=" ) relation } ;
relation    = shift     { ( "<" | ">" | "<=" | ">=" ) shift } ;
shift       = add       { ( "<<" | ">>" ) add } ;
add         = mul       { ( "+" | "-" ) mul } ;
mul         = unary     { ( "*" | "/" | "%" ) unary } ;
unary       = ( "!" | "~" | "-" | "&" | "*") unary | call ;
call        = primary { "(" args? ")" | "[" expr "]" | "." ident } ;
args        = expr { "," expr } ;
primary     = ident | lit | "(" expr ")" | block ;

ty          = path | "&" "mut"? ty | "*" ty | "[" ty ";" expr "]" | "(" ty { "," ty } ")" | "fn" "(" ty_list? ")" "->" ty ;
ty_list     = ty { "," ty } ;

path        = ident { "::" ident } ;
ident       = /* see lex */ ;
lit         = int_lit | float_lit | char_lit | string_lit | bool_lit ;
