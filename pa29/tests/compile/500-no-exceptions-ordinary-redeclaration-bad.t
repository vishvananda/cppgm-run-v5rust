// The allocation exception is narrow: ordinary functions still must agree.
void ordinary() noexcept;
void ordinary() noexcept(false);
