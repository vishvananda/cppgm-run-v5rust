#include "200-host-imported-covariant-return-adjustment.helper.h"

CovariantPrefix::~CovariantPrefix()
{
}

CovariantBase::~CovariantBase()
{
}

CovariantBase * CovariantBase::self()
{
  return this;
}

CovariantDerived::CovariantDerived() : value(9)
{
}

CovariantDerived * CovariantDerived::self()
{
  return this;
}

CovariantBase * make_covariant_base()
{
  static CovariantDerived instance;
  return &instance;
}
