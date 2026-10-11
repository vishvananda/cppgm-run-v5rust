// The SysV classes of direct object parameters and results (classes=,
// result_classes=) reach the object through the -O0 LowIR text, for direct,
// indirect and virtual calls.
struct SD { double d; long l; };
struct FF { float a, b, c; };
struct LD { long l; double d; };
struct AL { alignas(16) double d; };
struct LDB { long double x; };
struct E {};

__attribute__((noinline)) SD make(double d, long l) { SD s = {d, l}; return s; }
__attribute__((noinline)) double use(SD s, FF f, LD x, AL a, LDB b, E)
{
	return s.d + s.l + f.a + f.c + x.l + x.d + a.d + (double)b.x;
}
__attribute__((noinline)) LD mix(LD a, FF b) { LD r = {a.l + 1, a.d + b.b}; return r; }

struct Shape
{
	virtual LD area(LD scale) const { return scale; }
};
struct Square : Shape
{
	LD area(LD scale) const { LD r = {scale.l * 2, scale.d * 2}; return r; }
};

__attribute__((noinline)) double dispatch(const Shape &shape, LD (*through)(LD, FF))
{
	FF f = {1, 2, 3};
	LD a = shape.area(LD{3, 0.5});
	LD b = through(a, f);
	return b.l + b.d;
}

int main()
{
	SD s = make(1.5, 2);
	FF f = {1, 2, 3};
	Square square;
	return use(s, f, LD{4, 0.25}, AL{8}, LDB{16}, E()) == 35.75 &&
		dispatch(square, mix) == 10 ? 0 : 1;
}
