// N3485 5.3.7: declaration unwind facts do not change pointer type identity.
int (*safe)(int) noexcept;
int (*other)(int);
int main() { return noexcept(safe(2)) + noexcept(other(2)); }
