// Preserve the frozen syntax grammar before semantic signature validation.
struct Suffixes {
  Suffixes()[2];
  Suffixes()();
  operator int()[3];
};
Suffixes::Suffixes()[2] {}
