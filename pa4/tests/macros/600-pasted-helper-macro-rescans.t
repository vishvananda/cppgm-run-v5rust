#define CAT(a,b) CAT_I(a,b)
#define CAT_I(a,b) a ## b
#define CALL_0() CAT(A, B)
#define AB 1

CAT(CALL_, 0)()
