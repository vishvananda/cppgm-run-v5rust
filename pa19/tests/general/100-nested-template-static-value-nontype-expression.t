// VALIDATION: compile-pass
// Reduced from Boost.Container: evaluating a static const value through a
// nested qualified member class template must keep the structured template
// arguments for the outer qualifier.

typedef unsigned long size_t;

template<bool Loop>
struct upper_power_of_2_step;

template<>
struct upper_power_of_2_step<true> {
  template<class Integer, Integer I, Integer P>
  struct apply {
    static const Integer value =
        upper_power_of_2_step<(I > P * 2)>::template apply<Integer, I, P * 2>::value;
  };
};

template<>
struct upper_power_of_2_step<false> {
  template<class Integer, Integer I, Integer P>
  struct apply {
    static const Integer value = P;
  };
};

template<class Integer, Integer I>
struct upper_power_of_2_ct {
  static const Integer value =
      upper_power_of_2_step<(I > 1)>::template apply<Integer, I, 2>::value;
};

template<class SizeType,
         size_t HdrSize,
         size_t PayloadPerAllocation,
         size_t RealNodeSize,
         size_t NodeAlign>
struct calculate_alignment_ct {
  static const size_t proposed_alignment =
      upper_power_of_2_ct<SizeType,
                          HdrSize + PayloadPerAllocation + RealNodeSize>::value;
  static const size_t initial_alignment =
      NodeAlign > proposed_alignment ? NodeAlign : proposed_alignment;
};

template<size_t N>
struct take {
  static const size_t value = N;
};

static_assert(take<calculate_alignment_ct<size_t, 56, 8, 32, 8>::
                       initial_alignment>::value == 128,
              "nested template qualifier value should feed nontype arguments");
