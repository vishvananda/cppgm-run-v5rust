// Defining the key function demands the secondary template's body and
// by-value argument boundary even without constructing D; expect exit 0
struct Primary { virtual int first() { return 0; } };
struct Argument { int value; };
template<class T> struct Secondary {
    virtual int get(T t) { return t.value; }
};
struct D : Primary, Secondary<Argument> { virtual int key(); };
int D::key() { return 1; }
int main() { return 0; }
