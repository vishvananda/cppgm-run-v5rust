// expect exit 0
struct Member { Member() noexcept(false); };
struct Owner { Member member; Owner() noexcept = default; };
static_assert(!noexcept(Owner()), "computed constructor flag");
int main() { return 0; }
