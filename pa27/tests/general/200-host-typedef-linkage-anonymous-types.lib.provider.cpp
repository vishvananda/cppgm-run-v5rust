typedef struct { int value; } pa27_link_state;
typedef union { int value; long other; } pa27_link_union;
typedef enum { pa27_link_red = 4, pa27_link_blue = 9 } pa27_link_color;

namespace abi_link {
int struct_probe(pa27_link_state *state)
{
  return state->value == 11 ? 0 : 1;
}

int union_probe(pa27_link_union *value)
{
  return value->value == 13 ? 0 : 2;
}

int enum_probe(pa27_link_color color)
{
  return color == pa27_link_blue ? 0 : 4;
}
}
