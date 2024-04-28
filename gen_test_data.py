import hashlib
import os


def leading_zeros(num):
    if num == 0:
        return 8

    leading_zeros = 0
    while num & 0b10000000 == 0:
        leading_zeros += 1
        num = num << 1
        num = num & 0b11111111
    return leading_zeros


def total_leading_zeros(hash):
    to_return = 0
    for byte in hash:
        l_zeros = leading_zeros(byte)
        to_return += l_zeros
        if l_zeros < 8:
            break

    return to_return


def gen(hash, difficulty):
    difficulty = total_leading_zeros(difficulty)
    for i in range(1000):
        pow = b'' + os.urandom(10)
        hasher = hashlib.sha256()
        hasher.update(hash)
        hasher.update(pow)

        generated_hash = hasher.digest()
        ghash_leadin_zeros = total_leading_zeros(generated_hash)

        if ghash_leadin_zeros >= difficulty:
            print(pow, True)
        else:
            print(pow, False)


gen(hashlib.sha256(b'text').digest(),
    b'\x0F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF')
