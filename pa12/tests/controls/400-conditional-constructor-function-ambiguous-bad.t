// expect reject
struct Target;
struct Source { operator Target() const; };
struct Target { Target(const Source &); };
void inspect(bool choose, const Source & source, Target & target)
{
  (void)(choose ? source : target);
}
