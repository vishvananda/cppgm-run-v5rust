int a[2];
constexpr int* p = a + 2;
constexpr bool n = (const_cast<int*&>(p) += 1) == nullptr;
