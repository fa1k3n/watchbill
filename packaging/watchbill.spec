# The package that `dnf install` takes: the commands `wb` and `watchbill`, run by
# the Python of the distribution. packaging/rpm.sh builds it from a checkout and
# passes the revision as wb_version.
#
# The Rust `wb` is built and its tests are run here, and it is not installed yet: the
# installed `wb` is the Python one until #25. The package stays noarch for that reason.

Name:           watchbill
Version:        %{?wb_version}%{!?wb_version:0.0.1}
Release:        1%{?dist}
Summary:        A harness for AI agents in embedded software development

# No licence is chosen yet. This line says so, and is to be replaced when one is.
License:        LicenseRef-Not-Chosen
URL:            https://github.com/fa1k3n/watchbill
Source:         %{name}-%{version}.tar.gz

BuildArch:      noarch
BuildRequires:  python3-devel
BuildRequires:  pyproject-rpm-macros
# The tests read the two tables of the revision through the YAML library. The library
# `wb` runs with is named in setup.cfg, and the package asks for it from there.
BuildRequires:  python3-pyyaml
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
%pyproject_buildrequires
%cargo_generate_buildrequires

%build
%pyproject_wheel
%cargo_build

%install
%pyproject_install
%pyproject_save_files watchbill

%check
%{python3} -m unittest discover -s tests -v
%cargo_test

%files -f %{pyproject_files}
%doc README.md
%{_bindir}/wb
%{_bindir}/watchbill
