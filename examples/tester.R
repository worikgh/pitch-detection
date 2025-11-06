## Take the output of `tester.rs` and analyse it to find the best
## cnfigurations.  The best configurations get a result for all
## strings, and are ordered first by the minimum for the proportion
## that are "in tune" (10.0 < detected cents < 10.0, the `match`
## column) over each string, then by the mean over all strings.  It
## turns out that many different configurations get the same result
## for the B/3 string

library(dplyr)
library(kableExtra)
library(knitr)
library(purrr)
library(stringr)
library(tidyr)

parse_music_data <- function(file_path) {
  # Read the file
  lines <- readLines(file_path)

  # Initialize storage
  configurations <- list()
  test_cases <- list()
  results <- list()

  # Current configuration
  current_config <- NULL
  config_counter <- 0

  for (line in lines) {
    line <- trimws(line)
    if (line == "") next

    # Parse Detector Configuration line
    if (str_detect(line, "^Detector Configuration:")) {
      config_counter <- config_counter + 1
      config_parts <- str_split(line, "\\s+")[[1]]
      current_config <- list(
        config_id = config_counter,
        size = as.numeric(config_parts[3]),
        padding = as.numeric(config_parts[4]),
        power = as.numeric(config_parts[5]),
        clarity = as.numeric(config_parts[6]),
        method = config_parts[7]
      )
      configurations[[config_counter]] <- current_config
    }else if (str_detect(line, "^Test case")) {
      ## Parse Test case line
      test_parts <- str_split(line, "\\s+")[[1]]
      test_case <- list(
        config_id = config_counter,
        test_index = as.numeric(test_parts[3]),
        note = test_parts[4],  # e.g., "E/2"
        offset = as.numeric(test_parts[5]),
        file_path = test_parts[6]
      )
      test_cases[[length(test_cases) + 1]] <- test_case
    }else if (str_detect(line, "^Result:")) {
      # Parse Result line
      result_parts <- str_split(line, "\\s+")[[1]]

      # Extract note information (e.g., "E/E")
      note_info <- result_parts[3]
      note_parts <- str_split(note_info, "/")[[1]]
      actual_note <- note_parts[1]
      detected_note <- note_parts[2]


      # Extract octave information (e.g., "2/2")
      octave_info <- result_parts[4]
      octave_parts <- str_split(octave_info, "/")[[1]]
      actual_octave <- as.numeric(octave_parts[1])
      detected_octave <- as.numeric(octave_parts[2])

      # Extract cents information (e.g., "0.000/0.572")
      cents_info <- result_parts[5]
      cents_parts <- str_split(cents_info, "/")[[1]]
      actual_cents <- as.numeric(cents_parts[1])
      detected_cents <- as.numeric(cents_parts[2])

      result <- list(
        config_id = config_counter,
        result_index = as.numeric(result_parts[2]),
        actual_note = actual_note,
        detected_note = detected_note,
        actual_octave = actual_octave,
        detected_octave = detected_octave,
        actual_cents = actual_cents,
        detected_cents = detected_cents
      )
      results[[length(results) + 1]] <- result
    }
  }

  # Convert to data frames
  config_df <- do.call(rbind, lapply(configurations, as.data.frame))
  test_cases_df <- do.call(rbind, lapply(test_cases, as.data.frame))
  results_df <- do.call(rbind, lapply(results, as.data.frame))

  # Extract note and octave from test cases
  test_cases_df <- test_cases_df %>%
    mutate(
      actual_note = str_split(note, "/") %>% sapply(function(x) x[1]),
      actual_octave = str_split(note, "/") %>% sapply(function(x) as.numeric(x[2]))
    ) %>%
    select(-note)

  # Merge test case information with results
  analysis_df <- results_df %>%
    left_join(test_cases_df, by = c("config_id", "result_index" = "test_index")) %>%
    left_join(config_df, by = "config_id")

  (list(
    configurations = config_df,
    test_cases = test_cases_df,
    results = results_df,
    analysis_data = analysis_df
  ))
}

# Usage example:
# data_frames <- parse_music_data("your_data_file.txt")

# If you want to work with the text directly instead of a file:
parse_music_text <- function(text) {
  # Write text to temporary file and parse
  temp_file <- tempfile()
  writeLines(text, temp_file)
  result <- parse_music_data(temp_file)
  file.remove(temp_file)
  return(result)
}
data_frames <- parse_music_data("tester.log")
## # Example with your sample data
## sample_data <- "Detector Configuration:  1024  256  1.000  0.300 McLeod
## Test case   1 E/2  0.000 examples/data/E2_0.raw
## Result:   1 E/E 2/2  0.000/ 0.572
## Result:   1 E/E 2/2  0.000/ 2.272
## Result:   1 E/F 2/3  0.000/-44.262
## Test case   2 A/2  0.000 examples/data/A2_0.raw
## Result:   2 A/A 2/3  0.000/ 0.312
## Result:   2 A/A 2/2  0.000/ 0.473
## Test case   3 D/3  0.000 examples/data/D3_0.raw
## Result:   3 D/D 3/4  0.000/-32.501
## Result:   3 D/D 3/4  0.000/10.215
## Detector Configuration:  1024  256  1.000  0.500 McLeod
## Test case   1 E/2  0.000 examples/data/E2_0.raw
## Test case   2 A/2  0.000 examples/data/A2_0.raw
## Result:   2 A/A 2/2  0.000/ 2.406
## Result:   2 A/A 2/2  0.000/ 2.688
## Result:   2 A/A# 2/4  0.000/ 0.300"

## # Parse the sample data
## data_frames <- parse_music_text(sample_data)

# Access the data frames
configurations <- data_frames$configurations
test_cases <- data_frames$test_cases
results <- data_frames$results
analysis_data <- data_frames$analysis_data

## Return a list summarising the quality of a configuration
cfg_summary <- function(config_id) {
    ## Get the configuration for this configuration
    cc <- data_frames$configurations[cfg$config_id == config_id, ]

    ## The results for this configuration
    rr <- data_frames$results[r$config_id == config_id,]

    if(nrow(rr) == 0) {
        return (list(data.frame(),  -1 ))
    }
    ## Notes combined with octave as key    
    rr$actual_key <- paste0(rr$actual_note, "/", rr$actual_octave)

    ## Calculate how accurate each note's estimation is
    rr$match <- with(rr,
                     actual_note == detected_note &
                     actual_octave == detected_octave &
                     abs(detected_cents) < 10.0
                     )
    score <- aggregate(match ~ actual_key, data = rr, FUN = mean)
    names(score)[1] <- "actual_note"

    ## The actual notes reorted on by this configuration
    notes <- paste(rr$actual_note, rr$actual_octave, sep = "/")
    levels_order <- c("E/2", "A/2", "D/3", "G/3", "B/3", "E/4")
    notes_factor <- factor(notes, levels = levels_order)

    ## The occurence of the notes.  It is crucial all notes (strings)
    ## are represented fairly
    freq <- as.data.frame(table(notes_factor))
    names(freq) <-  c("actual_note","frequency")
    merged <- merge(freq, score, by = "actual_note", all.x = TRUE)
    merged$actual_note <- factor(merged$actual_note, levels = levels_order)
    merged <- merged[order(merged$actual_note), ]
    row.names(merged) <- NULL

    ## The threashold
    score <- min(merged$match, na.rm = FALSE)
    if(is.na(score)) {
        score <- -1
    }
    mean_score <- mean(merged$match, na.rm = FALSE)
    if(is.na(mean_score)) {
        mean_score <- -1
    }
    params <- paste0(cc$method," size/", cc$size, " power/", cc$power,  " clarity/", cc$clarity,  " padding/", cc$padding )
    list(merged, score, mean_score, params)
}
ids <- data_frames$configurations$config_id
results <- lapply(ids, cfg_summary) 
 get_or_na <- function(x, i) if (length(x) >= i) x[[i]] else NA_real_

v2 <- sapply(results, get_or_na, 2)
v3 <- sapply(results, get_or_na, 3)

# put NAs last
ord <- order(is.na(v2), v2, is.na(v3), v3)
results_sorted <- results[ord]

print_result <- function(x, index = NULL, title = NULL, digits = 2) {
  if (!is.null(title)) cat("==", title, "==\n")
  if (!is.null(index)) cat("Item:", index, "\n")

  print(kable(x[[1]], digits = digits, align = "lrr"))
  cat(sprintf("\nMinimum: %.*f  Mean: %.*f\n", digits, as.numeric(x[[2]]), digits, as.numeric(x[[3]])))
  cat("Info:", as.character(x[[4]]), "\n")
  cat(strrep("-", 60), "\n\n")
}

# Iterate over all list elements and print a pretty block for each.
# Adjust indices if your list contains other types or structure.
walk2(results_sorted, seq_along(results_sorted), ~{
  # Only print elements that follow the expected structure (a list of 4)
  if (is.list(.x) && length(.x) >= 4) {
    print_result(.x, index = .y)
  }
})

