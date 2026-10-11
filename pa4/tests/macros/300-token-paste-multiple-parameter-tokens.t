#define triple(a,b,c) a ## b ## c
triple(a, A.B, b)

#define call_triple(a,b,c) triple(a,b,c)
call_triple(x, X Y, y)
