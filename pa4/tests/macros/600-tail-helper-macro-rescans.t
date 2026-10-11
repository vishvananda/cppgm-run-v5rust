#define CAT(a, b) CAT_I(a, b)
#define CAT_I(a, b) a ## b

#define FILLER_0(...) (__VA_ARGS__) FILLER_1
#define FILLER_1(...) (__VA_ARGS__) FILLER_0
#define FILLER_0_END
#define FILLER_1_END

CAT(FILLER_0(a)(b)(c), _END)
