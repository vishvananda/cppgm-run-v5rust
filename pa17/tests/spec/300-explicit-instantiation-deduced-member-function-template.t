// VALIDATION: compile-pass
// N3485 focus: 14.7.2 [temp.explicit]

struct Length {};
struct Filter {};

struct Graph
{
  template<class Evaluator, class Predicate>
  int search(Evaluator const&, Predicate const&) const;
};

template<class Evaluator, class Predicate>
int Graph::search(Evaluator const&, Predicate const&) const
{
  return 7;
}

template int Graph::search(Length const&, Filter const&) const;

int main()
{
  return 0;
}
