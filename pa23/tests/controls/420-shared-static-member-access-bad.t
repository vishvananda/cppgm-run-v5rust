// expect reject
struct V { static int value; };
struct U : virtual V { private: using V::value; };
struct W : virtual V { private: using V::value; };
struct D : U, W {};
int main() { return sizeof(D::value); }
