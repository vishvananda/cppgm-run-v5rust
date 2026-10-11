struct Source {};
struct Target
{
  explicit Target(const Source &);
};

void inspect(bool first, Source source, const Target & target)
{
  // Failed direct reference binding does not permit an explicit constructor.
  (void)(first ? source : target);
}
