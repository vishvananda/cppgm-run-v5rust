enum error_type {
  error_collate_v = 1
};

constexpr error_type error_collate(error_collate_v);
static_assert(error_collate == error_collate_v, "bad");

int main() {
  return 0;
}
