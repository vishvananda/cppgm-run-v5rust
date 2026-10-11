// A virtual function overriding one of a non-primary base has its own entry
// in the class's primary vtable (Itanium 2.5.2), the one a call through the
// class uses: D::x overrides X::x in the virtual base V, and V::y overrides
// Y::y of its secondary base Y.  The H classes repeat the hierarchy with
// their functions, and so their vtables, built by the host compiler.
struct X { virtual int x(); int xv; };
struct Y { virtual int y(); int yv; };
struct V : X, Y { int vv; virtual int y(); };
struct W { virtual int w(); int wv; };
struct D : W, virtual V { int dv; virtual int x(); };

struct HX { virtual int x(); int xv; };
struct HY { virtual int y(); int yv; };
struct HV : HX, HY { int vv; virtual int y(); };
struct HW { virtual int w(); int wv; };
struct HD : HW, virtual HV { int dv; virtual int x(); };

int host_d_x(D& value);
int host_v_y(V& value);
int host_v_x(V& value);
HD* host_make();
