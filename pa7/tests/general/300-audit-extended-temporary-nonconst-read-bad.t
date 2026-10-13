const int& reference = 3;
constexpr int n = const_cast<int&>(reference);
