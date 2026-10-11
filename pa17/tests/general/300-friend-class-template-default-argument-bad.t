// N3485 14.1p9: a default template-argument shall not be specified in a friend
// class template declaration (clang rejects, g++ accepts); expect reject
class X { template <class T = int> friend class Y; };
int main() { return 0; }
