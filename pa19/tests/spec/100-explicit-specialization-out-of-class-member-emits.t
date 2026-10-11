// VALIDATION: emit-lowir
// N3485 focus: 14.7.3 [temp.expl.spec]

template<class T>
struct explicit_specialization_member_emits
{
  void value(T);
};

template<>
struct explicit_specialization_member_emits<char>
{
  void value(char);
};

void explicit_specialization_member_emits<char>::value(char)
{
}
