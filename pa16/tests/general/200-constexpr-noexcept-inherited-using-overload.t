struct hash_code {
  hash_code() noexcept {}
};

struct node {
  node() noexcept {}
};

struct base {
  int bucket(hash_code, unsigned long) { return 0; }
  int bucket(node, unsigned long) noexcept { return 0; }
};

struct access : base {
  using base::bucket;
};

access &declval_access() noexcept;

static_assert(noexcept(declval_access().bucket(node(), 0)),
              "inherited noexcept overload should be visible");

int main()
{
  return 0;
}
