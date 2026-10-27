struct Left { int value; };
struct Right { int value; };
struct Derived : Left, Right {
    Derived() : value(1) {}
};
