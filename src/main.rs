#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(non_snake_case)]
mod Transaction;
mod Token;
mod Tools;
use num_bigint::BigUint;
mod merkletree;
mod Block;
use base64;
use rsa::{RsaPublicKey, 
        pkcs1::FromRsaPublicKey, 
        PaddingScheme, 
        hash::Hash::SHA2_256,
        RsaPrivateKey,
        pkcs1::FromRsaPrivateKey};
use rsa::PublicKey;
use sha2::{Sha256, Digest};
use std::convert::TryInto;
mod BlockChainTree;


static PRIVATE_KEY:&'static str = "MIIJKQIBAAKCAgEAsHRYAlZ5jokPIYr76MWr0i+IOohQ7321OJ7GRjj+1+Ffsby/\
                TMcsG4qywTH92WD6qvva9kYyBsL4ji/UYGP2r1jevwLoeyF8AQPUauTfFdndC3Qr\
                /0GdEXaxP+OLJxCwV5YFZrZBNlmri46rFwNHK1GWFxsrWr9IqoN2cuagQLLtgbyK\
                kdHQgo26g6zqSnyYAtGKnUAXye/9H4Ygnpdt5ep8o25ZmjzZTKkLwmKw8JUaIqM3\
                5YQOtBlfk6XVfLrNMjkqvCUpTQCMGlRNh37Z6LOWUfwn9YRTvXkcSTHnXw/IkVj3\
                RmSWyy2PaUch3Yewm9STR9o4y1OJ6b8fyJtOm1LM4Jxf4VVGmY2iu7xgIY89OgLy\
                Qt8TTlAtc4jm1BkC0yi7ckQJPkS/1dz4Knj33fA+qyKORsjwuNMdEOKbc0mzWM+q\
                /v/UAajqO0T/YEHs3KRHxF5EiAENaP61X2RWEiekmCPMs+1isgjzGusoYc3XfvLV\
                g8lQtaDj+VU7oJCVGLyAUtk3X54bk/UHX48Nd28MxCCEB8aD0Fb2PIbLNiyIjpvM\
                7+L6EfaCBTTouCDYPA8I5O083YI6A4is5P7sG8FGJkE1HCE2LAPHabz/mbuTEVbG\
                bE8GHQ4w/vcCikHaNY6Voo+tmyDG/Dd/lzIRivMYLZc2mFvWU2So2VhoX2sCAwEA\
                AQKCAgAgkKg5bjoq2xKmzx6km+6U8N7EZqoBOQL7+xl+lkA6GUbu53WqtstmHdfV\
                neNKfgJlEewcLWJIKy1yGFOqomhBfjZFrES5jLl0+n72Z1zgjH+mAH+qMTLKqUwX\
                DWh+Ai95Sp+ekB5C6JOnQuJCcgFtdjYr94Ct08WWaoKuu+9xYVRB8EjmloFTxtJY\
                QM5QhuQLCR763HxHcXoWmktk2W+X0gnEo7X3qCk2Y7Be5JXhILNdFyBWxGNuS4x/\
                FEjtJxB9p11Csd0IcARrS4mvavLOZm5knRPG1PAbdMGKpKIdwizy3idsn13RC4x/\
                EnimiMdISxgPV6/JnpZCRFqaTEdaRpmjfRVeeX/qSiFBFTpcqegqWjX60hlPDEtc\
                CG2gtxqq9E2p6TG7NqGQw2gD/xkjrxKF3VPWSoaRmPZIsrzNR7XhKcGr/8ZI7oKm\
                C67O6/m3Svi7I6vvQ7ZPLJQ5wjzJZRHrSr4AVWLGKZvx4oSIOLOF6bLRHRlPKGnb\
                Budd1epSi8o1cXHj28EY2TxGBidRS9SpPO+PncFJMrIZAODzE2oA6oQAUmxrdvpi\
                QR45F6oyf7DCeEinqTlUTYsDEJQlk5NiBKwj4aZ04/7w3zm/pERhhWjPVpnEuXx2\
                KmxhJ50y6ip8NoXPRfWacpcWWRY6S+hZlfI5XNAGbUywMr1rEQKCAQEAz3HDle6C\
                edmk33aTZXD1qpDd6ChAbYO5WuaoEUN+Jy+yWW+gBVXp2lb3L/e+X/3c2nvCUmCT\
                DrFABssFwh7C96Pl0uUEM32D+SFL/gpp+QUhQcsRGxi2d5EDg7FP84yVRNwqcCzl\
                8B/AX7tD90FqRIxuqfT/IoFl7+jDMaBPStATdAodM7Xjnqjxb7Wq65cYvWfPZc/X\
                joNmZQPjxgqiNcQidAaMk1EvUxOSoXIhwexbo9Trc44g5JtmMI90+RQU9A+nKfOX\
                rNJ+CRyYKZHDKFseelcltOc8+Ob4dH/jZMBFdLrQANC4nt2v5K8VtcNrAyJwyq4n\
                jVCUjaoykwZIeQKCAQEA2cGjdHHCyJRvNUMYxcUeC470fX/NOy7qKKMiVg4dLwcs\
                biyuj/wRYEE3zjG638imeEidbIPlq8twQdBudRjkSvdoU9pTPZGXjjmRMDTu8eEU\
                iqNYulU4WMALbCuR1UVF5nWJI8EYx+vmE0GhAzeoCGC+2vKebuxYZXYndIPfDlqx\
                +OI2q3etnjEoBwU6axc4YXglhmLmbZ3YiWGp8gGJZP82yRS/PwzPr8Y3k48Xx9rr\
                XSS7VOo8JFxwdE0XDPN5rJN8fH0LhJLz1no0JDCWVi0Ng2Ids3mVvlt+m1jxaN9h\
                1o9xcweEWIjr+QoDtwHAI2EJmnAHrPIA2Vd5QWw2AwKCAQEAnscivvWp5H4T1f66\
                XuCRCJaNYw68EZbLHqqBZYVVX8UAK7hmsO7LaZU66fokOvDiRFCJsee9Z3d/3DM1\
                GxnUfRtz43HrP0YI53z232E1L6cfh25Yj3bg4q+aEwh6e53U+rnRub6D2MFUy3FY\
                Wj41inY6ldeyGMUWMwTjsm5Tgp205hJ/u36y1FPXSHuycVRbWU3FztXA1ZH5o8PQ\
                aVrmQWT4QfppSrDPGjVW/D+RWw8ALWvhM7dLse7Hzs9e5u7aAtygRFnwdBVA5tR+\
                GwM4bwEfWOCvOcHsR07ySKlCcXFBOFFst9MKHH7uDIl+gnsqw2FvF5MpLt7IdY4Z\
                27LpcQKCAQEAxW9hoamnvy9+aV9trZtc84PpP4TJ8xhFbRUEg9wGL+akLTzMBXa5\
                1nkrfQPv+Qk3jqXgPkyUyCALp1CxZfBsxV/vMuAoSxGfxW+CqZ/E6oB2nIEgaMnt\
                7eIqOSiD8Ef/6cW10zo4GsRTdjAyKfWjn/z/7wq+Bbq7Jztq6KTMcHphFVUd1ngL\
                bfwJ29usrP5/uzgxZdh0Lv1IIL/xU6B7D5yq8sSh5ivafvgM2fiKykU+09QDGinK\
                3/kEaR6ggLidIJEU9NZ3w/ttpJBmiE2ZcYcl0nL6lQjvcYoJkBajw7+OOPkUFwTA\
                xXlX4xRma97lc+5+w41CGFfP10ANi/juOwKCAQAt9bz2fKNCZi0n/99Pv85Lf/HS\
                YFp8B1h3vO8wrNgl1Adf6b93lAV5rU+L+x/Jpanh+f+dSfrz4l4N1Sudtz/4KGnn\
                Z5W+rlYpe4QBSfN4l8Y8pU6dwWtCM4QthlIs87gtMQoBvx/pPQPgZe4a6k9zvSGJ\
                5nlxoYuW0dpfHKJvsh5dvidj4m0Lxi3kVFKp6GlsYdu4kKMnIjcW6bFITz8tCBvT\
                16eBWeT4e+eMssWIHhPbp65v4tjvJh/AzIDoVaODMbkZ9A/kOLDX0t4yGJ7Mquga\
                eyVMTlDZQVN96YN6XIQ+9j5CXbm7xWaWivRfNNSEfydUtSt+sjVgBNKMlS0v";

static PUBLIC_KEY:&'static str = "MIICCgKCAgEAsHRYAlZ5jokPIYr76MWr0i+IOohQ7321OJ7GRjj+1+Ffsby/TMcs\
                G4qywTH92WD6qvva9kYyBsL4ji/UYGP2r1jevwLoeyF8AQPUauTfFdndC3Qr/0Gd\
                EXaxP+OLJxCwV5YFZrZBNlmri46rFwNHK1GWFxsrWr9IqoN2cuagQLLtgbyKkdHQ\
                go26g6zqSnyYAtGKnUAXye/9H4Ygnpdt5ep8o25ZmjzZTKkLwmKw8JUaIqM35YQO\
                tBlfk6XVfLrNMjkqvCUpTQCMGlRNh37Z6LOWUfwn9YRTvXkcSTHnXw/IkVj3RmSW\
                yy2PaUch3Yewm9STR9o4y1OJ6b8fyJtOm1LM4Jxf4VVGmY2iu7xgIY89OgLyQt8T\
                TlAtc4jm1BkC0yi7ckQJPkS/1dz4Knj33fA+qyKORsjwuNMdEOKbc0mzWM+q/v/U\
                AajqO0T/YEHs3KRHxF5EiAENaP61X2RWEiekmCPMs+1isgjzGusoYc3XfvLVg8lQ\
                taDj+VU7oJCVGLyAUtk3X54bk/UHX48Nd28MxCCEB8aD0Fb2PIbLNiyIjpvM7+L6\
                EfaCBTTouCDYPA8I5O083YI6A4is5P7sG8FGJkE1HCE2LAPHabz/mbuTEVbGbE8G\
                HQ4w/vcCikHaNY6Voo+tmyDG/Dd/lzIRivMYLZc2mFvWU2So2VhoX2sCAwEAAQ==";

static PREVIOUS_HASH:[u8;32] = [1,1,1,1,1,1,1,1,
                            1,1,1,1,1,1,1,1,
                            1,1,1,1,1,1,1,1,
                            1,1,1,1,1,1,1,1];

fn main() {
    // let decoded_private_key = base64::decode(PRIVATE_KEY).unwrap();
    // let private_key = RsaPrivateKey::from_pkcs1_der(&decoded_private_key).unwrap();


    // let current_owner:String = String::from("CurrentOwner");
    // let previous_owner:String = String::from(PUBLIC_KEY);
    // let token_data:String = String::from("TokenData");
    // let smol_contract:String = String::from("SmolContract");
    // let coin_supply:BigUint = BigUint::from(32768u64);
    // let transfer_fee:BigUint = BigUint::from(228u64);

    // let mut hasher = Sha256::new();
    // //hasher.update(&current_owner);
    // hasher.update(PREVIOUS_HASH);
    // let token_hash:[u8;32] = hasher.finalize().as_slice().try_into().unwrap();

    // let mut data_to_sign:Vec<u8> = Vec::new();
    // data_to_sign.push(1);

    // for byte in previous_owner.as_bytes().iter(){
    //     data_to_sign.push(*byte);
    // }
    // for byte in current_owner.as_bytes().iter(){
    //     data_to_sign.push(*byte);
    // }
    // for byte in token_data.as_bytes().iter(){
    //     data_to_sign.push(*byte);
    // }
    // for byte in smol_contract.as_bytes().iter(){
    //     data_to_sign.push(*byte);
    // }
    
    // let coin_supply_as_string:String = coin_supply.to_str_radix(10);
    // let coin_supply_as_bytes:&[u8] = coin_supply_as_string.as_bytes();
    // for byte in coin_supply_as_bytes{
    //     data_to_sign.push(*byte);
    // }

    // let transfer_fee_as_string:String = transfer_fee.to_str_radix(10);
    // let transfer_fee_as_bytes:&[u8] = transfer_fee_as_string.as_bytes();
    // for byte in transfer_fee_as_bytes{
    //     data_to_sign.push(*byte);
    // }

    // for byte in token_hash.iter(){
    //     data_to_sign.push(*byte);
    // }

    // let mut hasher = Sha256::new();
    // hasher.update(data_to_sign);
    // let data_to_sign_hash:[u8;32] = hasher.finalize().as_slice().try_into().unwrap();

    // let padding_scheme = PaddingScheme::new_pkcs1v15_sign(Some(SHA2_256));
    // let signed_data = private_key.sign(padding_scheme,
    //                                     &data_to_sign_hash).unwrap();
    // let signature:String = base64::encode(signed_data);

    // // let mut token:Token::Token = Token::Token::new(current_owner.clone(),
    // //                                             previous_owner.clone(),
    // //                                             signature.clone(),
    // //                                             token_hash.clone(),
    // //                                             token_data.clone(),
    // //                                             smol_contract.clone(),
    // //                                             coin_supply.clone(),
    // //                                             transfer_fee.clone(),
    // //                                             true).unwrap();
    
    // // let res = token.verify().unwrap();
    // // println!("{:?}",res);

    // let transaction:Transaction::Transaction = Transaction::Transaction::new(previous_owner,
    //                                             current_owner,
    //                                             228,
    //                                             signature,
    //                                             coin_supply);

    // let dump = transaction.dump().unwrap();
    // let tr = Transaction::Transaction::parse_transaction(&dump[1..], dump.len() as u64 -1);
    // println!("{:?}",tr);
    // //println!("{:?}",transaction.verify(&PREVIOUS_HASH))
    
    let data:[u8;9] = [1,2,3,4,5,6,7,8,9];
    BlockChainTree::compress_to_file(String::from("dump_test.txt"),
                             &data);

    let res = BlockChainTree::decompress_from_file(String::from("dump_test.txt"));
    println!("{:?}",res);
}
