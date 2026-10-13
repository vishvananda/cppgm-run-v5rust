constexpr int assigned = (const_cast<int&>(static_cast<const int&>(3)) = 4);
constexpr int compound = (const_cast<int&>(static_cast<const int&>(3)) += 4);
constexpr int prefix = ++const_cast<int&>(static_cast<const int&>(3));
constexpr int postfix = const_cast<int&>(static_cast<const int&>(3))++;
constexpr int fresh = (const_cast<int&>(static_cast<const int&>(3)) = 4, const_cast<int&>(static_cast<const int&>(3)));
constexpr int wrap = (const_cast<unsigned char&>(static_cast<const unsigned char&>(255)) += 1);
int f() { return assigned + compound + prefix + postfix + fresh + wrap; }
