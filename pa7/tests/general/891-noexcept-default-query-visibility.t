// N3485 5.3.7: the immediately preceding default is visible in a specification.
int default_value();
int safe(int = default_value()) noexcept;
int queried() noexcept(noexcept(safe())) { return 3; }
int main() { return noexcept(safe()) + noexcept(safe(1)); }
