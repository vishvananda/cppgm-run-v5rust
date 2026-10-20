// expect exit 0
struct V {
    static int value;
    static int f(int) { return 7; }
    static int f(long) { return 8; }
    template<class T> static T identity(T x) { return x; }
};
int V::value = 3;
struct U : virtual V {
private:
    using V::value;
    using V::f;
    using V::identity;
};
struct W : virtual V {};
struct D : U, W {};
struct Reverse : W, U {};
struct Protected {
protected:
    static int member;
};
int Protected::member = 9;
struct Public : virtual Protected { using Protected::member; };
struct Other : virtual Protected {};
struct Reintroduced : Other, Public {};
int main() {
    int (*selected)(int) = &D::f;
    if (sizeof(D::value) != sizeof(int) || D::value != 3) return 1;
    if (Reverse::value != 3 || D::f(0) != 7 || Reverse::f(0L) != 8) return 2;
    if (selected(0) != 7 || D::identity<int>(4) != 4) return 3;
    if (Reintroduced::member != 9) return 4;
    return 0;
}
