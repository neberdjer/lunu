use std::fmt::Write;

pub struct OpfBook<'a> {
	pub title: &'a str,
	pub author: Option<&'a str>,
	pub series_name: Option<&'a str>,
	pub series_sequence: Option<&'a str>,
	pub asin: Option<&'a str>,
}

pub fn metadata_opf(book: &OpfBook<'_>) -> String {
	let mut out = String::with_capacity(512);
	out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
	out.push_str(
		"<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"2.0\" \
		 unique-identifier=\"bookid\">\n",
	);
	out.push_str(
		"\t<metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
		 xmlns:opf=\"http://www.idpf.org/2007/opf\">\n",
	);

	let _ = writeln!(out, "\t\t<dc:title>{}</dc:title>", escape(book.title));
	if let Some(author) = nonblank(book.author) {
		let _ = writeln!(
			out,
			"\t\t<dc:creator opf:role=\"aut\">{}</dc:creator>",
			escape(author)
		);
	}
	if let Some(asin) = nonblank(book.asin) {
		let _ = writeln!(
			out,
			"\t\t<dc:identifier id=\"bookid\" opf:scheme=\"ASIN\">{}</dc:identifier>",
			escape(asin)
		);
	}
	if let Some(series) = nonblank(book.series_name) {
		let _ = writeln!(
			out,
			"\t\t<meta name=\"calibre:series\" content=\"{}\"/>",
			escape(series)
		);
		if let Some(sequence) = nonblank(book.series_sequence) {
			let _ = writeln!(
				out,
				"\t\t<meta name=\"calibre:series_index\" content=\"{}\"/>",
				escape(sequence)
			);
		}
	}

	out.push_str("\t</metadata>\n</package>\n");
	out
}

fn nonblank(value: Option<&str>) -> Option<&str> {
	value.map(str::trim).filter(|value| !value.is_empty())
}

fn escape(value: &str) -> String {
	let mut out = String::with_capacity(value.len());
	for c in value.chars() {
		match c {
			'&' => out.push_str("&amp;"),
			'<' => out.push_str("&lt;"),
			'>' => out.push_str("&gt;"),
			'"' => out.push_str("&quot;"),
			'\'' => out.push_str("&apos;"),
			_ => out.push(c),
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	fn book<'a>(title: &'a str) -> OpfBook<'a> {
		OpfBook {
			title,
			author: None,
			series_name: None,
			series_sequence: None,
			asin: None,
		}
	}

	#[test]
	fn a_bare_book_still_produces_a_parseable_package() {
		let opf = metadata_opf(&book("The Hobbit"));
		assert!(opf.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
		assert!(opf.contains("<dc:title>The Hobbit</dc:title>"));
		assert!(opf.trim_end().ends_with("</package>"));
		assert!(
			!opf.contains("<dc:creator"),
			"an absent author must be omitted, not written empty"
		);
	}

	#[test]
	fn markup_in_a_title_cannot_break_out_of_the_document() {
		let opf = metadata_opf(&book("Tom & Jerry <script>\"x\""));
		assert!(opf.contains("<dc:title>Tom &amp; Jerry &lt;script&gt;&quot;x&quot;</dc:title>"));
		assert!(
			!opf.contains("<script>"),
			"an unescaped title would corrupt every downstream parser"
		);
	}

	#[test]
	fn audiobookshelf_reads_the_fields_it_expects() {
		let opf = metadata_opf(&OpfBook {
			author: Some("J.R.R. Tolkien"),
			series_name: Some("Middle-earth"),
			series_sequence: Some("1"),
			asin: Some("B08G9PRS1K"),
			..book("The Hobbit")
		});
		assert!(opf.contains("<dc:creator opf:role=\"aut\">J.R.R. Tolkien</dc:creator>"));
		assert!(opf.contains("<meta name=\"calibre:series\" content=\"Middle-earth\"/>"));
		assert!(opf.contains("<meta name=\"calibre:series_index\" content=\"1\"/>"));
		assert!(opf.contains("opf:scheme=\"ASIN\">B08G9PRS1K</dc:identifier>"));
	}

	#[test]
	fn a_sequence_without_a_series_is_meaningless_and_omitted() {
		let opf = metadata_opf(&OpfBook {
			series_sequence: Some("3"),
			..book("Orphan")
		});
		assert!(!opf.contains("calibre:series_index"));
	}

	#[test]
	fn blank_fields_are_treated_as_absent() {
		let opf = metadata_opf(&OpfBook {
			author: Some("   "),
			..book("The Hobbit")
		});
		assert!(!opf.contains("<dc:creator"));
	}
}
