struct Member {
  ~Member() {}
};

int main()
{
  const auto fn = []()
  {
    struct Candidate {
      Member member;
    };
    Candidate candidate;
    return 0;
  };
  return fn();
}
