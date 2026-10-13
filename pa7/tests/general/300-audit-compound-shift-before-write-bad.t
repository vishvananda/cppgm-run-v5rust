const int x = 3;
constexpr int n = (const_cast<int&>(x) <<= 32);
