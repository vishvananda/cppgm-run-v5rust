struct V { virtual int tag(); };
struct D : virtual V { virtual int id(); int tag(); };
int D::id() { return 1; }
