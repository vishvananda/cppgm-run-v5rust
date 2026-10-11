#define EAT(x)
#define REM(x) x
#define LPAREN() (
#define RPAREN() )
#define COMMA() ,
#define A(m,e) m(LPAREN() e COMMA() A_ID)
#define A_ID() A
#define B(m,e) m(RPAREN() B_ID)
#define B_ID() B
A (REM,4)() (REM,1)() (EAT,?) NIL B (REM,4)() (REM,1)() (EAT,?)
