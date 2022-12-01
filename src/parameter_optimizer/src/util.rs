use std::process::Command;

pub fn run_waf_command(
    path: &str,
    command: &str,
    env: std::collections::HashMap<&str, &str>,
) -> Result<(), crate::Error> {
    let mut waf_path = std::fs::canonicalize(path).unwrap();
    waf_path.push("waf");
    let arg = format!("{} {}", waf_path.to_str().unwrap(), command);
    println!("Running: {}", arg);

    if Command::new("bash")
        .current_dir(path)
        .arg("-c")
        .arg(arg)
        .envs(env)
        .spawn()?
        .wait()?
        .success()
    {
        Ok(())
    } else {
        Err(format!("Failed to run command: waf {}", command).into())
    }
}

/// Macro that creates a map from key value pairs in the following format:
/// ("Key1" => 1, "Key2" => 2)
#[macro_export]
macro_rules! map {
    // map-like
    ($($k:expr => $v:expr),* $(,)?) => {
        std::iter::Iterator::collect(std::array::IntoIter::new([$(($k, $v),)*]))
    };
}

fn lerp<T, F>(a: T, b: T, f: F) -> T
where
    T: Copy,
    T: std::ops::Sub<Output = T>,
    T: std::ops::Add<Output = T>,
    T: std::ops::Mul<F, Output = T>,
{
    //Convert the 0-1 range into a value in the right range.
    a + ((b - a) * f)
}

fn normalize<T, F>(a: T, b: T, value: T) -> F
where
    T: Copy,
    T: std::ops::Sub<Output = T>,
    T: std::ops::Div<Output = F>,
{
    (value - a) / (b - a)
}

pub fn map<S, D, F>(left_min: S, left_max: S, value: S, right_min: D, right_max: D) -> D
where
    S: Copy,
    S: std::ops::Sub<Output = S>,
    S: std::ops::Div<Output = F>,
    D: Copy,
    D: std::ops::Sub<Output = D>,
    D: std::ops::Add<Output = D>,
    D: std::ops::Mul<F, Output = D>,
{
    //Figure out how 'wide' each range is
    let f: F = normalize(left_min, left_max, value);

    lerp(right_min, right_max, f)
}

pub struct RangeSmoother<T>
where
    T: Copy + From<i32>,
{
    data: Vec<T>,
}

impl<T> RangeSmoother<T>
where
    T: Copy + From<i32>,
{
    pub fn new(steps: usize, values: &[T]) -> Self {
        let mut data: Vec<T> = Vec::with_capacity(steps);
        for i in 0..steps {
            let mut j = 0;
            let src_index = i * values.len() / steps;
            let src_index = loop {
                //Find a value of j that causes src_index to increase by one.
                //Then return the index before that, so we include all lower values
                let new_src_index = (i + j) * values.len() / steps;
                if new_src_index > src_index {
                    break new_src_index - 1;
                }
                j += 1;
            };
            data.push(values[src_index]);
        }

        Self { data }
    }

    pub fn ranges(&self) -> impl Iterator<Item = T> + '_ {
        self.data.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_smooth() {
        let smoother = RangeSmoother::new(4, &[0, 5, 6, 7]);
        let ranges: Vec<i32> = smoother.ranges().collect();
        assert_eq!(ranges.as_slice(), &[0, 5, 6, 7]);

        let smoother = RangeSmoother::new(2, &[0, 5, 6, 7]);
        let ranges: Vec<i32> = smoother.ranges().collect();
        assert_eq!(ranges.as_slice(), &[5, 7]);

        let smoother = RangeSmoother::new(3, &[0, 5, 6, 7, 10]);
        let ranges: Vec<i32> = smoother.ranges().collect();
        assert_eq!(ranges.as_slice(), &[0, 6, 10]);

        let smoother = RangeSmoother::new(16, &[0, 5, 6, 7, 10, 24, 25, 26]);
        let ranges: Vec<i32> = smoother.ranges().collect();
        assert_eq!(
            ranges.as_slice(),
            &[0, 0, 5, 5, 6, 6, 7, 7, 10, 10, 24, 24, 25, 25, 26, 26]
        );

        let smoother = RangeSmoother::new(8, &[10, 15, 25]);
        let ranges: Vec<i32> = smoother.ranges().collect();
        assert_eq!(ranges.as_slice(), &[10, 10, 10, 15, 15, 15, 25, 25]);
    }
}
