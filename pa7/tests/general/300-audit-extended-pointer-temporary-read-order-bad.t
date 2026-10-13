int*const& reference = nullptr;
constexpr bool n = (const_cast<int*&>(reference) += 1) == nullptr;
