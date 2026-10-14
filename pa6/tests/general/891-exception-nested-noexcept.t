// N3485 5.3.7 and 15.4: nested unevaluated call query is a bool constant.
void safe() noexcept;
void yes() noexcept(noexcept(safe()));
void no() noexcept(!noexcept(safe()));
