struct FloatPair { float a; float b; double c; };
struct FP8 { float a; float b; };
double sse_pair_sum(struct FloatPair p) { return p.a + p.b + p.c; }
float fp8_sum(struct FP8 p) { return p.a + p.b; }
extern double our_take16(struct FloatPair p);
extern float our_take8(struct FP8 p);
int drive(void)
{
	struct FloatPair p16 = {1.0f, 2.0f, 39.0};
	struct FP8 p8 = {40.0f, 2.0f};
	if (our_take16(p16) != 42.0) return 3;
	if (our_take8(p8) != 42.0f) return 4;
	return 0;
}
