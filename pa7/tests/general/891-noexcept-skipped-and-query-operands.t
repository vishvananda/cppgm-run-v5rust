// N3485 5.3.7: both logical/conditional branches count; nested queries do not.
int other();
int main() { return noexcept(false && other()) + noexcept(true ? 0 : other()) + noexcept(sizeof(other())) + noexcept(noexcept(other())); }
