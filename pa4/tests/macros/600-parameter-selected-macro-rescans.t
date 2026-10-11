#define IIF(c,t,f) IIF_I(c,t,f)
#define IIF_I(c,t,f) IIF_ ## c(t,f)
#define IIF_0(t,f) f
#define IIF_1(t,f) t
#define NODE_1(p) IIF(p(1), 1, 2)
#define P(n) 1
#define CAT(a,b) CAT_I(a,b)
#define CAT_I(a,b) a ## b

CAT(REPEAT_, IIF(1, NODE_1, NODE_2)(P))
