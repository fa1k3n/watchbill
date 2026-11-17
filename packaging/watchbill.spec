# The package that `dnf install` takes: the commands `wb` and `watchbill`, built from Rust, and
# the files of the revision under /usr/share/watchbill. packaging/rpm.sh builds it from a
# checkout and passes the version of Cargo.toml as wb_version: the spec has none of its own.

Name:           watchbill
Version:        %{wb_version}
Release:        1%{?dist}
Summary:        A harness for AI agents in embedded software development

License:        Apache-2.0
URL:            https://github.com/fa1k3n/watchbill
Source:         %{name}-%{version}.tar.gz

# The tests make git repositories.
BuildRequires:  git-core
# The Rust of the distribution, and the macros that build with it and take every crate
# from the distribution's packages. On Enterprise Linux 9 the macros and the crates are in
# EPEL, so the build needs EPEL there. The macros ask for the crates of Cargo.toml when they
# generate the build requirements; each crate is also named here, so that a build without
# them says so at once.
BuildRequires:  cargo
BuildRequires:  rust >= 1.97
BuildRequires:  cargo-rpm-macros
# The YAML library the Rust `wb` reads every YAML file with. 0.10.3 does not compile without
# its default feature, which Cargo.toml turns off.
BuildRequires:  crate(yaml-rust2) >= 0.10.4
# The SHA-256 of the manifest of the revision. 0.10.9 is the oldest that builds with the
# features Cargo.toml names.
BuildRequires:  crate(sha2) >= 0.10.9
# `wb init` asks git for the root of the repository and the address of origin.
Requires:       git-core

%description
Watchbill is a harness for AI agents in embedded software development, deployed
at a fixed revision onto a project that already exists. The command is wb.

%prep
%autosetup -n %{name}-%{version}
# Points cargo at the crates of the distribution and the network off, as .cargo/config.toml does.
%cargo_prep

%generate_buildrequires
%cargo_generate_buildrequires

%build
# The folder of the revision is fixed in the program here, and only here: the tests that run
# after the build read the files of the checkout.
export WB_SHARE=%{_datadir}/watchbill
%cargo_build

%install
install -D -m 0755 target/rpm/wb %{buildroot}%{_bindir}/wb
ln -s wb %{buildroot}%{_bindir}/watchbill
mkdir -p %{buildroot}%{_datadir}/watchbill
cp -a share/watchbill/. %{buildroot}%{_datadir}/watchbill/

%check
%cargo_test

# One line for each kind of file: a file of the revision that is not named here is not packaged,
# and rpmbuild stops at it. A test holds the manifest to these lines (WB-REV-22).
%files
%license LICENSE NOTICE
%doc README.md
%{_bindir}/wb
%{_bindir}/watchbill
%dir %{_datadir}/watchbill
%dir %{_datadir}/watchbill/revision
%dir %{_datadir}/watchbill/revision/roles
%{_datadir}/watchbill/revision/roles/*.md
%{_datadir}/watchbill/revision/*.yml
%{_datadir}/watchbill/revision.sha256
%{_datadir}/watchbill/revisions.txt
