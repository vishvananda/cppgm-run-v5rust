#ifndef CPPGM_PA13_NONPRIMARY_CHAINED_MEMBER_TEMPLATE_LAMBDA_ANCHOR_COLLISION_H
#define CPPGM_PA13_NONPRIMARY_CHAINED_MEMBER_TEMPLATE_LAMBDA_ANCHOR_COLLISION_H

namespace nonprimary_chained_lambda
{

template<class T = void>
struct second
{
  ~second()
  {
  }

  template<class F>
  int postcondition(F const& f)
  {
    f();
    return 0;
  }
};

template<class T = void>
struct first
{
  template<class F>
  second<T> old(F const&)
  {
    return second<T>();
  }
};

extern int side_effect;

inline int run()
{
  return first<void>()
      .old([] { side_effect = 1; })
      .postcondition([] { side_effect = 2; });
}

}

#endif
