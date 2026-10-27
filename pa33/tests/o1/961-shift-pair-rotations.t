# Complementary shifts retain the frozen rotation admission and direction.

function @left(%x : i64) -> i64 {
block ^entry:
  %l = binary shl i64 %x, 13
  %r = binary ushr i64 %x, 51
  %value = binary or i64 %l, %r
  return i64 %value
}

function @right(%x : i64) -> i64 {
block ^entry:
  %l = binary shl i64 %x, 51
  %r = binary ushr i64 %x, 13
  %value = binary xor i64 %r, %l
  return i64 %value
}

function @tie(%x : i64) -> i64 {
block ^entry:
  %l = binary shl i64 %x, 32
  %r = binary ushr i64 %x, 32
  %value = binary add i64 %l, %r
  return i64 %value
}

function @u32left(%x : u32) -> u32 {
block ^entry:
  %l = binary shl u32 %x, 7
  %r = binary ushr u32 %x, 25
  %value = binary add u32 %l, %r
  return u32 %value
}

function @u32right(%x : u32) -> u32 {
block ^entry:
  %l = binary shl u32 %x, 25
  %r = binary ushr u32 %x, 7
  %value = binary or u32 %r, %l
  return u32 %value
}

function @one(%x : i64) -> i64 {
block ^entry:
  %l = binary shl i64 %x, 1
  %r = binary ushr i64 %x, 63
  %value = binary xor i64 %r, %l
  return i64 %value
}

function @shared(%x : i64) -> i64 {
block ^entry:
  %l = binary shl i64 %x, 13
  %r = binary ushr i64 %x, 51
  %v = binary or i64 %l, %r
  %value = binary add i64 %v, %l
  return i64 %value
}
function @signed32(%x : i32) -> i32 {
block ^entry:
  %l = binary shl i32 %x, 7
  %r = binary ushr i32 %x, 25
  %v = binary or i32 %l, %r
  return i32 %v
}
function @separate(%x : i64) -> i64 {
block ^entry:
  %l = binary shl i64 %x, 13
  %r = binary ushr i64 %x, 51
  jump ^next
block ^next:
  %v = binary or i64 %l, %r
  return i64 %v
}

function @main() -> i64 [role=entry] {
block ^entry:
  %rleft = call i64 @left(1)
  %badleft = cmp ne i64 %rleft, 8192
  %rright = call i64 @right(1)
  %badright = cmp ne i64 %rright, 2251799813685248
  %rtie = call i64 @tie(1)
  %badtie = cmp ne i64 %rtie, 4294967296
  %ru32left = call u32 @u32left(1)
  %badu32left = cmp ne u32 %ru32left, 128
  %ru32right = call u32 @u32right(1)
  %badu32right = cmp ne u32 %ru32right, 33554432
  %rone = call i64 @one(1)
  %badone = cmp ne i64 %rone, 2
  %rshared = call i64 @shared(1)
  %badshared = cmp ne i64 %rshared, 16384
  %rsigned32 = call i32 @signed32(1)
  %badsigned32 = cmp ne i32 %rsigned32, 128
  %rseparate = call i64 @separate(1)
  %badseparate = cmp ne i64 %rseparate, 8192
  %badall0 = binary or i1 %badleft, %badright
  %badall1 = binary or i1 %badall0, %badtie
  %badall2 = binary or i1 %badall1, %badu32left
  %badall3 = binary or i1 %badall2, %badu32right
  %badall4 = binary or i1 %badall3, %badone
  %badall5 = binary or i1 %badall4, %badshared
  %badall6 = binary or i1 %badall5, %badsigned32
  %badall7 = binary or i1 %badall6, %badseparate
  return i64 %badall7
}
