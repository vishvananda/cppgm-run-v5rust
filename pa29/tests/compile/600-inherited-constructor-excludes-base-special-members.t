struct base {
  base(int);
  base(const base&);
  base(base&&);
};

struct target : base { using base::base; };
struct source { operator target(); };

static_assert(__is_constructible(target, source&), "");

// N3485 12.9: the other subobjects must be default-initializable.
struct deleted_member { deleted_member() = delete; };
struct invalid_target : base {
  using base::base;
  deleted_member member;
};

static_assert(__is_constructible(target, int), "inherited constructor");
static_assert(!__is_constructible(invalid_target, int), "deleted member");

int main() {}
