// Types mode retains the function body without typing its return occurrence.
constexpr int value() { return 1; }
void query() noexcept(value());
