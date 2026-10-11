let-expr N template-param 0
let-expr words_per_word literal 64
let-expr word_index binary dv N words_per_word
let-expr remainder binary rm N words_per_word
let-expr zero literal 0
let-expr one literal 1
let-expr has_no_tail binary eq remainder zero
let-expr tail_words conditional has_no_tail zero one
let-expr word_count binary pl word_index tail_words
let-arg word_count_arg expression word_count
type template std::_Base_bitset word_count_arg
