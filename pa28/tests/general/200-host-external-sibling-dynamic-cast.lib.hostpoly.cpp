#include "200-host-external-sibling-dynamic-cast.helper.h"

struct HiddenDerived : HiddenLeft, HiddenRight {
};

HiddenLeft::~HiddenLeft()
{
}

HiddenRight::HiddenRight() : payload(19)
{
}

HiddenRight::~HiddenRight()
{
}

HiddenLeft * make_hidden_left()
{
  static HiddenDerived instance;
  return &instance;
}
