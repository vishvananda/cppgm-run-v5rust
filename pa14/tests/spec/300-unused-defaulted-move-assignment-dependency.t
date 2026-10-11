// N3485 focus: 12.8 [class.copy], 14.7.1 [temp.inst]
// Merely declaring a defaulted move assignment for a class-template
// specialization does not demand the unused dependent base assignment body.

template<class T>
struct assignment_base
{
  assignment_base& operator=(assignment_base&&)
  {
    (void)sizeof(T);
    return *this;
  }
};

template<class T>
struct assignment_owner : assignment_base<T>
{
  assignment_owner& operator=(assignment_owner&&) = default;
};

struct incomplete;

assignment_owner<incomplete> value;

int main()
{
  return sizeof(value) == 1 ? 0 : 1;
}
