// VALIDATION: compile-pass

const unsigned int align_only = 1u << 0u;
const unsigned int size_ordered = 1u << 1u;
const unsigned int address_ordered = 1u << 2u;

template<unsigned Flags>
struct impl
{
  static const unsigned value = Flags;
};

template<unsigned OverheadPercent>
struct pool
  : impl<unsigned(OverheadPercent == 0) * align_only |
         size_ordered |
         address_ordered>
{
};

static_assert(pool<1>::value == 6, "functional cast expression argument");
