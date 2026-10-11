// VALIDATION: compile-pass

template<class T>
struct accepts_primary_definition
{
  static const bool value = false;
};

struct defined_special {};
struct external_special {};

struct transformer
{
  template<class T>
  int transform(T&);
};

int complete_transformer_owner[sizeof(transformer)];

template<>
int transformer::transform<defined_special>(defined_special&);

template<>
int transformer::transform<external_special>(external_special&);

template<class T>
int transformer::transform(T&)
{
  static_assert(accepts_primary_definition<T>::value,
                "explicit specialization declarations do not use this body");
  return 0;
}

template<>
int transformer::transform<defined_special>(defined_special&)
{
  external_special value;
  return transform<external_special>(value);
}

int use(transformer& object, defined_special& value)
{
  return object.transform(value);
}
