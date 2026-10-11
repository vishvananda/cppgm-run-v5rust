// Triviality does not override an explicit destructor exception specification.
struct Safe {};
struct Throwing { ~Throwing() noexcept(false) = default; };
Safe* receiver();
Safe* safe_receiver() noexcept;
Throwing* throwing_receiver() noexcept;
static_assert(!noexcept(receiver()->~Safe()), "receiver may throw");
static_assert(noexcept(safe_receiver()->~Safe()), "safe receiver and destructor");
static_assert(!noexcept(throwing_receiver()->~Throwing()), "declared throwing destructor");
int main() { return 0; }
